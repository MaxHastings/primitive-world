"""Joint juvenile opportunity funnel: all stages refer to actual individual lives."""
import json
import sys
from pathlib import Path
from summarize_endowment import distribution


def summarize(root):
    arms=[]
    for d in sorted(root.glob('seed-*')):
        if not d.is_dir() or not (d/'completion.json').exists():
            continue
        rows=[json.loads(p.read_text()) for p in d.glob('world-*.json')]
        children=[c for r in rows for c in r['funnel']['individuals'].values()]
        def ticks(c,field): return c.get('contact_opportunity',{}).get(field,0)
        stages={
            'births':len(children),
            'ever_near_organism':sum(ticks(c,'nearby_organism_ticks')>0 for c in children),
            'ever_near_food_with_headroom':sum(ticks(c,'nearby_food_ticks')>0 for c in children),
            'ever_near_funded_transfer_intent':sum(ticks(c,'nearby_funded_transfer_intent_ticks')>0 for c in children),
            'ever_received_food':sum(c['juvenile_received_milli']>0 for c in children),
            'matured':sum(c['maturity_tick'] is not None for c in children)}
        arms.append({'arm':d.name,'model':rows[0]['model'],'experimental_legacy_fraction_gate':rows[0].get('experimental_legacy_fraction_gate',False),'experimental_actual_energy_gate':rows[0].get('experimental_actual_energy_gate',False),'population':rows[0]['settings']['population'],'stages':stages,
            'near_food_ticks':distribution([ticks(c,'nearby_food_ticks') for c in children]),
            'near_intent_ticks':distribution([ticks(c,'nearby_funded_transfer_intent_ticks') for c in children]),
            'received_food':distribution([c['juvenile_received_milli']/1000 for c in children]),
            'highest_contact_lives':sorted(children,key=lambda c:ticks(c,'nearby_food_ticks'),reverse=True)[:5]})
    return {'completed_arms':len(arms),'arms':arms,'scope':'Per-child evidence, not a product of unrelated aggregate rates. Contact is measured before arbitration and does not imply the child is the chosen receiver. Nearby food counts require receiver headroom. A juvenile dying during body upkeep has no contact phase on that terminal tick. Births alive at the horizon are censored.'}


if __name__=='__main__':
    print(json.dumps(summarize(Path(sys.argv[1])),indent=2))
