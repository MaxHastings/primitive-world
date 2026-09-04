"""Recurrent PPO over the ordinary GPU world; diagnostics are read-only."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import numpy as np
import torch
from policy import Policy, OBS, HIDDEN, ACTIONS, VERSION

ROOT = Path(__file__).resolve().parents[1]

class World:
    def __init__(self, exe):
        self.log = open(ROOT / 'training' / 'runtime.log', 'a', encoding='utf8')
        self.process = subprocess.Popen([str(Path(exe).resolve()), '--headless', '--neural-bridge'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.log, text=True, encoding='utf8', bufsize=1, creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
        self.ready = self.read()
        assert self.ready['schema'] == VERSION and self.ready['interval'] == 8
        assert (self.ready['observations'], self.ready['hidden'], self.ready['actions']) == (OBS,HIDDEN,ACTIONS)

    def read(self):
        line = self.process.stdout.readline()
        if not line:
            raise RuntimeError(f'GPU bridge ended ({self.process.poll()}); see training/runtime.log')
        result = json.loads(line)
        if 'error' in result:
            raise RuntimeError(result['error'])
        return result

    def call(self, **message):
        self.process.stdin.write(json.dumps(message,separators=(',',':'))+'\n')
        self.process.stdin.flush()
        return self.read()

    def close(self):
        if self.process.poll() is None:
            self.call(op='quit')
            self.process.wait(timeout=20)
        self.log.close()

def collect(world, population, seed, steps, blind, memory_reset=False, greedy=False, forget_cue=False, setup=None):
    reset = dict(op='reset', population=population, seed=int(seed), blind=blind,
                 memory_reset=memory_reset, neural=True, greedy=greedy)
    if setup:
        reset.update(setup)
    world.call(**reset)
    frames=[]
    for t in range(steps+1):
        if forget_cue and t==1:world.call(op='forget')
        frames.append(world.call(op='step'))
        if t==steps-1:
            outcome=world.call(op='metrics')
    for t, frame in enumerate(frames):
        assert frame['tick'] == t*8 and frame['elapsed_ticks']==8
        for i,row in enumerate(frame['rows']):
            if row['valid']:
                tr=row['trace']
                assert row['slot']==i and tr['tick']==frame['tick'] and tr['generation']==row['generation']
    # The extra frame supplies a value bootstrap, not another evaluation step.
    frames[-1]['world_outcome'] = outcome
    return frames

def tensors(frames, reward):
    field=lambda fn: torch.tensor([[fn(r) for r in f['rows']] for f in frames],dtype=torch.float32)
    obs=field(lambda r:r['trace']['observation'])
    masks=field(lambda r:r['trace']['mask'])
    # Dead rows are loss-masked; give their categorical a valid dummy action.
    invalid=masks.sum(-1)==0
    masks[...,0][invalid]=1
    return dict(obs=obs, masks=masks,
                done=field(lambda r:r['done']),valid=field(lambda r:r['valid']),
                action=field(lambda r:r['trace']['choice']).long(),
                old_logp=field(lambda r:np.log(max(r['trace']['probabilities'][r['trace']['choice']],1e-30))),
                reward=field(lambda r:r['reward_'+reward]),
                gpu_logits=field(lambda r:r['trace']['logits']),gpu_hidden=field(lambda r:r['trace']['after']))

def summarize(frames):
    rows=[r for f in frames for r in f['rows'] if r['valid']]
    final=frames[-1]['rows']
    # Food harvested is reserve-equivalent improvement plus actual costs, not a reward target.
    return dict(sampled_slot_occupancy=float(np.mean([r['alive']!=0 for r in final])),
                energy=float(np.mean([r['energy'] for r in final])),
                food=float(np.mean([r['food'] for r in final])),
                physiology_return=float(sum(r['reward_physiology'] for r in rows)/len(final)),
                reached_food=float(np.mean([any(f['rows'][i]['valid'] and f['rows'][i]['trace']['observation'][2]>.001 for f in frames) for i in range(len(final))])))

def evaluate(world, model, args):
    world.call(op='weights',weights=model.export('evaluation'))
    results=[];parity=0.
    for blind in [False,True]:
        for mode in ['recurrent','forget_cue','memory_reset']:
            samples=[]
            for seed in [args.eval_seed,args.eval_seed+18,args.eval_seed+42]:
                full=collect(world,args.population,seed,args.eval_steps,blind,mode=='memory_reset',not args.eval_stochastic,forget_cue=mode=='forget_cue')
                outcome=full[-1]['world_outcome'];fs=full[:-1]
                if mode=='recurrent':
                    batch=tensors(fs,'physiology')
                    with torch.no_grad():
                        _,_,_,logits,hidden=model.sequence(batch['obs'],batch['masks'],batch['done'])
                        valid=batch['valid'].bool()
                        error=max((logits[valid]-batch['gpu_logits'][valid]).abs().max().item(),(hidden[valid]-batch['gpu_hidden'][valid]).abs().max().item())
                        if error>2e-4:raise RuntimeError(f'Exported evaluation policy mismatch: {error}')
                        parity=max(parity,error)
                row=summarize(fs)
                row.update(world_living=outcome['world']['living'],world_births=outcome['world']['events'][3],max_ancestry_depth=outcome['evolution']['maximum_generation'])
                samples.append(dict(seed=seed,**row))
            results.append(dict(blind=blind,mode=mode,mean={k:float(np.mean([s[k] for s in samples])) for k in samples[0] if k!='seed'},seeds=samples))
    return results,parity

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--exe',default=str(ROOT/'target/release/primitive_world.exe'))
    ap.add_argument('--updates',type=int,default=80)
    ap.add_argument('--population',type=int,default=64)
    ap.add_argument('--steps',type=int,default=192)
    ap.add_argument('--eval-steps',type=int,default=192)
    ap.add_argument('--seed',type=int,default=7)
    ap.add_argument('--reward',choices=['physiology','survival'],default='survival',
                    help='Survival is the default; physiology is an explicit reserve-scoring experiment')
    ap.add_argument('--output',default=str(ROOT/'reports/experimental-gru-v3.json'))
    ap.add_argument('--load')
    ap.add_argument('--resume',help='Resume complete actor/critic PyTorch state')
    ap.add_argument('--eval-seed',type=int,default=100001)
    ap.add_argument('--eval-stochastic',action='store_true',help='Sample actions as in the default live neural mode')
    ap.add_argument('--no-evaluation',action='store_true',help='Training stage only; evaluate the final exported model separately')
    ap.add_argument('--evaluate-only',action='store_true')
    args=ap.parse_args()
    if not 1 <= args.population <= 256 or args.steps < 2 or args.eval_steps < 1 or args.updates < 1:
        ap.error('population must be 1..256, steps >=2, eval-steps and updates >=1')
    torch.set_num_threads(1);torch.manual_seed(args.seed);np.random.seed(args.seed)
    model=Policy()
    if args.load:model.load_actor(json.loads(Path(args.load).read_text()))
    if args.resume:model.load_state_dict(torch.load(args.resume,weights_only=True))
    initial_recurrent=model.gru.weight_hh.detach().clone()
    initial_input=model.gru.weight_ih.detach().clone()
    optimizer=torch.optim.Adam(model.parameters(),lr=3e-4)
    world=World(args.exe);history=[];start=time.perf_counter();max_parity=0.
    output=Path(args.output);output.parent.mkdir(parents=True,exist_ok=True)
    try:
        if not args.evaluate_only:
            # Every rollout uses the ordinary world. These are parameterized
            # weather/social conditions, not substitute maps or reward hacks.
            setups = [
                {},
                {'regeneration': 0.005},
                {'regeneration': 0.02},
                {'static_landscape': True},
                {'no_social': True, 'no_force': True},
            ]
            for update in range(args.updates):
                world.call(op='weights',weights=model.export(f'ppo-{args.reward}-{update}'))
                setup = setups[update % len(setups)]
                fs=collect(world,args.population,args.seed*10000+update,args.steps,blind=update%2==0,setup=setup)
                batch=tensors(fs,args.reward);T=args.steps
                with torch.no_grad():
                    lp,values,_,logits,hidden=model.sequence(batch['obs'],batch['masks'],batch['done'])
                    valid=batch['valid'].bool()
                    err=max((logits[valid]-batch['gpu_logits'][valid]).abs().max().item(),(hidden[valid]-batch['gpu_hidden'][valid]).abs().max().item())
                    max_parity=max(max_parity,err)
                    if err>2e-4:raise RuntimeError(f'GPU/Python recurrent mismatch: {err}')
                    expected_lp=lp.gather(-1,batch['action'][...,None]).squeeze(-1)
                    if (expected_lp[valid]-batch['old_logp'][valid]).abs().max()>2e-4:raise RuntimeError('Behavior policy probability mismatch')
                    advantages=torch.zeros_like(values[:T]);gae=torch.zeros(args.population)
                    for t in reversed(range(T)):
                        continuation=1-batch['done'][t]
                        delta=batch['reward'][t]+.99*values[t+1]*continuation-values[t]
                        gae=delta+.99*.95*continuation*gae
                        advantages[t]=gae
                    returns=advantages+values[:T]
                    active=batch['valid'][:T].bool()
                    if active.sum()<2:raise RuntimeError('Insufficient valid life transitions; no update was applied')
                    advantage=(advantages-advantages[active].mean())/(advantages[active].std()+1e-8)
                for epoch in range(4):
                    lp,new_values,entropy,_,_=model.sequence(batch['obs'][:T],batch['masks'][:T],batch['done'][:T])
                    logp=lp.gather(-1,batch['action'][:T,...,None]).squeeze(-1)
                    ratio=(logp-batch['old_logp'][:T]).exp()
                    policy_loss=-torch.minimum(ratio*advantage,ratio.clamp(.8,1.2)*advantage)[active].mean()
                    value_loss=.5*(new_values-returns).square()[active].mean()
                    loss=policy_loss+.5*value_loss-.01*entropy[active].mean()
                    optimizer.zero_grad();loss.backward();grad=torch.nn.utils.clip_grad_norm_(model.parameters(),.5)
                    if not torch.isfinite(grad):raise RuntimeError('Nonfinite gradient')
                    optimizer.step()
                row=dict(update=update, setup=setup, seconds=time.perf_counter()-start,
                         parity=err,loss=float(loss.detach()),**summarize(fs[:T]))
                history.append(row)
                if update%5==0 or update+1==args.updates:
                    print(json.dumps(row),flush=True)
                    output.write_text(json.dumps(model.export(f'ppo-{args.reward}-{update+1}')),encoding='utf8')
            if not (model.gru.weight_hh-initial_recurrent).abs().max()>0 or not (model.gru.weight_ih-initial_input).abs().max()>0:
                raise RuntimeError('Training did not update input and recurrent parameters')
            torch.save(model.state_dict(),output.with_suffix('.pt'))
        evaluation,eval_parity=([],0.) if args.no_evaluation else evaluate(world,model,args)
        max_parity=max(max_parity,eval_parity)
        actor_bytes=json.dumps(model.export('fingerprint'),sort_keys=True).encode()
        report=dict(config=vars(args),torch_version=torch.__version__,actor_sha256=hashlib.sha256(actor_bytes).hexdigest(),optimizer_resume=False,seconds=time.perf_counter()-start,max_python_gpu_error=max_parity,recurrent_parameter_change=float((model.gru.weight_hh-initial_recurrent).abs().max()),input_parameter_change=float((model.gru.weight_ih-initial_input).abs().max()),history=history,evaluation=evaluation)
        output.with_suffix('.report.json').write_text(json.dumps(report,indent=2),encoding='utf8')
        print(json.dumps(dict(evaluation=evaluation,seconds=report['seconds'])),flush=True)
    finally:
        world.close()

if __name__=='__main__':main()
