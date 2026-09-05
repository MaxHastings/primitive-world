"""Read-only Primitive World checkpoint audit; no simulation or checkpoint edits.

Examines cumulative counters and a sufficient mathematical test for action
suppression. A failed suppression test does NOT prove the action is reachable.
Requires NumPy; refuses unknown layouts. Output is exclusively created.
"""
import argparse
import hashlib
import json
import struct
from pathlib import Path

import numpy as np


N, H, G = 16384, 16, 2646
ACTIONS = ["none", "collect", "transfer", "force", "emit", "reproduce"]


def dtype(spec):
    return np.dtype([(name, '<' + kind, shape) if shape else (name, '<' + kind)
                     for name, kind, shape in spec])


AGENT = dtype([
    ('position','f4',2),('velocity','f4',2),('energy','f4',()),('age','f4',()),
    ('max_speed','f4',()),('sensor_radius','f4',()),('food','f4',()),
    ('action','u4',()),('target','u4',()),('alive','u4',()),('body_padding','f4',()),
    ('rng','u4',()),('generation','u4',()),('next_birth','u4',()),('max_age','f4',()),
    ('signal_payload','f4',()),('signal_tick','u4',()),('signal_padding','u4',3),
    ('collected','f4',()),('ingested','f4',()),('spent','f4',()),('received','f4',()),
    ('moved','f4',2),('lineage_id','u4',()),('parent_lineage','u4',()),
    ('birth_tick','u4',()),('birth_parent_slot','u4',()),('ancestry_depth','u4',()),
    ('lifetime_births','u4',()),('distance_travelled','f4',()),('founder_family','u4',()),
    ('hidden','f4',16),('mutation_probability','f4',()),('mutation_magnitude','f4',())])
DECISION = dtype([
    ('scores','f4',6),('selected_action','u4',()),('score_padding','u4',()),
    ('movement','f4',2),('amount','f4',()),('payload','f4',()),('target','u4',()),
    ('target_generation','u4',()),('invalid','u4',()),('body_padding','u4',()),
    ('force','f4',2),('mutation_probability','f4',()),('mutation_magnitude','f4',()),('hidden','f4',16),('update_gates','f4',16),('inputs','f4',108)])


def suppression(genomes):
    out = genomes[:,2272:].reshape(-1,22,17).astype(np.float64)
    result = {}
    for action in [2,3,4]:
        # For h in [-1,1]^16, min(score_rival-score_action) = db - sum(abs(dw)).
        delta = out[:,:6,:] - out[:,action:action+1,:]
        lower = delta[:,:,16] - np.abs(delta[:,:,:16]).sum(axis=2)
        lower[:,action] = -np.inf
        best = lower.max(axis=1)
        result[ACTIONS[action]] = {
            'genomes': len(genomes),
            'provably_outscored_for_every_hidden_state': int((best > 1e-5).sum()),
            'best_rival_guaranteed_margin_min': float(best.min()),
            'best_rival_guaranteed_margin_median': float(np.median(best)),
            'best_rival_guaranteed_margin_max': float(best.max()),
        }
    return result


def audit(path):
    raw = path.read_bytes()
    assert raw[:12] == b'PRIMWORLD017', 'Expected primitive-world checkpoint 17'
    seed,tick,size = struct.unpack_from('<III',raw,12)
    settings = json.loads(raw[24:24+size]); pos=24+size; buffers=[]
    expected=[N*216,512*512*4,512*512*4,512*512*32,128,65536*40,N*400,N*640,N*G*4]
    for length in expected:
        actual=struct.unpack_from('<Q',raw,pos)[0]; pos+=8
        assert actual == length, (actual,length)
        buffers.append(memoryview(raw)[pos:pos+actual]); pos+=actual
    assert pos == len(raw), 'Truncated or trailing checkpoint data'
    assert AGENT.itemsize==216 and DECISION.itemsize==640
    agents=np.frombuffer(buffers[0],dtype=AGENT)
    stats=np.frombuffer(buffers[4],dtype='<u4').astype(np.uint64)
    decisions=np.frombuffer(buffers[7],dtype=DECISION)
    genomes=np.frombuffer(buffers[8],dtype='<f4').reshape(N,G)
    assert np.isfinite(genomes).all() and (np.abs(genomes)<=4).all()
    alive=agents['alive']==1; slots=np.flatnonzero(alive)
    assert len(slots)>0
    assert int(settings['population']) + int(stats[3])-int(stats[1])-int(stats[2])-int(stats[7]) == len(slots), 'Population accounting mismatch'
    # Saved decisions are from the preceding step. Exclude births which can reuse
    # a slot with a stale decision; verify surviving bodies match decision state.
    valid=alive & (agents['birth_tick'] < tick-1)
    assert np.array_equal(agents['hidden'][valid],decisions['hidden'][valid])
    out=genomes[valid,2272:].reshape(-1,22,17).astype(np.float64)
    recomputed=np.einsum('noh,nh->no',out[:,:6,:16],decisions['hidden'][valid].astype(np.float64))+out[:,:6,16]
    error=float(np.max(np.abs(recomputed-decisions['scores'][valid])))
    assert error<1e-4, ('Saved GPU score parity failed',error)
    assert np.array_equal(recomputed.argmax(axis=1),decisions['selected_action'][valid])
    received=decisions['inputs'][valid][:,[58,65,72,79,86,93,100,107]]
    founders=np.asarray(settings.pop('founder_genomes'),dtype=np.float32)
    result={
        'checkpoint':str(path.resolve()),'checkpoint_sha256':hashlib.sha256(raw).hexdigest(),
        'model':'primitive-v5','checkpoint_schema':17,'seed':seed,'tick':tick,
        'settings_without_genomes':settings,'living':len(slots),
        'births':int(stats[3]),'starvation_deaths':int(stats[1]),'age_deaths':int(stats[2]),
        'emissions':int(stats[9]),'completed_transfers':int(stats[4]),'completed_force':int(stats[5]),
        'action_selections':dict(zip(ACTIONS,map(int,stats[24:30]))),
        'invalid_outputs':int(stats[31]),
        'signal_counter_cannot_have_wrapped':tick*N<2**32,
        'living_with_nonzero_last_emission_tick':int((agents['signal_tick'][alive]>0).sum()),
        'retained_slots_with_nonzero_last_emission_tick':int((agents['signal_tick']>0).sum()),
        'living_action_counts':dict(zip(ACTIONS,map(int,np.bincount(agents['action'][alive],minlength=6)))),
        'living_ancestry_depth':{'min':int(agents['ancestry_depth'][alive].min()),'max':int(agents['ancestry_depth'][alive].max())},
        'saved_decision_validation':{'matched_living_agents':int(valid.sum()),'max_gpu_score_absolute_error':error,
            'all_selected_actions_match':True,'receiver_decisions_with_present_signal':int((received>0).any(axis=1).sum())},
        'current_living_action_suppression':suppression(genomes[alive]),
        'stored_initial_founder_action_suppression':suppression(founders),
        'limits':[
            'Historical counters describe this saved world since its last reset, not future behavior or other worlds.',
            'Emission count records successful sends, not deliveries or useful communication.',
            'No historical receiver trace is reconstructed from this single checkpoint.',
            'Suppression is a sufficient bound over all hidden states; a nonpositive bound does not demonstrate reachability.',
            'No weights, rules, settings, or checkpoint bytes are modified.'
        ]}
    assert path.read_bytes()==raw, 'Checkpoint changed during audit'
    return result


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('checkpoint',type=Path)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    result=audit(args.checkpoint)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('x',encoding='utf-8') as f:
        json.dump(result,f,indent=2,allow_nan=False);f.write('\n')
    print(json.dumps({k:result[k] for k in ['tick','living','emissions','action_selections','current_living_action_suppression','saved_decision_validation']},indent=2))
