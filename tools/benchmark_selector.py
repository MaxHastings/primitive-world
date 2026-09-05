"""Frozen matched-population comparisons on held-out pools and seeds."""
import argparse
import json
from pathlib import Path
import subprocess


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exe",type=Path,required=True)
    parser.add_argument("--pool",type=Path,required=True)
    parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--seeds",type=int,nargs="+",required=True)
    parser.add_argument("--ticks",type=int,default=200000)
    parser.add_argument("--population",type=int,default=1000)
    parser.add_argument("--compositions",type=int,default=3)
    parser.add_argument("--population-model",type=Path)
    parser.add_argument("--individual-model",type=Path)
    args=parser.parse_args()
    if args.ticks<=0 or not 2<=len(args.seeds)<=32 or len(set(args.seeds))!=len(args.seeds):
        parser.error("Use a positive total budget and 2..32 distinct seeds")
    args.output.mkdir(parents=True,exist_ok=False)
    results=[]
    for policy,model in [("uniform",None),("individual",args.individual_model),("population",args.population_model)]:
        folder=(args.output/policy).resolve()
        command=[str(args.exe.resolve()),"--train-loop",str(folder),"--single-batch",
                 "--candidate-pool",str(args.pool.resolve()),"--selector-policy",policy,"--selector-frozen",
                 "--comparison-seeds",",".join(map(str,args.seeds)),"--compositions",str(args.compositions),
                 "--population",str(args.population),"--ticks",str(args.ticks)]
        if model: command += ["--selector-model",str(model.resolve())]
        subprocess.run(command,check=True)
        report=json.loads((folder/"comparison.json").read_text())
        results.append(dict(policy=policy,model=str(model) if model else "untrained",report=report))
    result=dict(total_tick_budget_per_policy=args.ticks,comparisons=results,
                scope="Frozen policies use a shared held-out pool and seed list. Incomplete batches are resumable and do not support completed-batch mean comparisons. Missing models are untrained controls.")
    (args.output/"comparison.json").write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps({r["policy"]:r["report"]["completed"] for r in results}))


if __name__=="__main__":
    main()
