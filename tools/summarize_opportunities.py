"""Summarize completed opportunity arms without selecting or modifying populations."""
import json
import sys
from pathlib import Path
from summarize_endowment import summarize


def summarize_factorial(root):
    result=[]
    for directory in sorted(root.glob('seed-*')):
        if not directory.is_dir() or not (directory/'completion.json').exists():
            continue
        summary=summarize(directory)
        rows=[json.loads(p.read_text()) for p in directory.glob('world-*.json')]
        settings=rows[0]['settings']
        keys=list(rows[0]['funnel']['opportunities'])
        opportunities={k:sum(r['funnel']['opportunities'][k] for r in rows) for k in keys if k!='scope'}
        result.append({'arm':directory.name,'model':rows[0]['model'],'experimental_legacy_fraction_gate':rows[0].get('experimental_legacy_fraction_gate',False),'experimental_actual_energy_gate':rows[0].get('experimental_actual_energy_gate',False),'population':settings['population'],'regeneration_setting':settings['resource_regeneration'],
            'ticks':summary['elapsed_ticks_reported'],'births':summary['births'],'matured':summary['matured'],
            'birth_energy':summary['birth_energy'],'manufactured_packets':summary['packet_manufacture_energy']['count'],
            'large_manufactured_packets':summary['packets_over_32_energy'],
            'opportunities':opportunities,
            'harvested_food':sum(r['metrics']['harvested'] for r in rows),
            'ingested_food':sum(r['metrics']['food_ingested'] for r in rows),
            'regenerated_food':sum(r['metrics']['regenerated'] for r in rows),
            'capacity_blocked_packets':sum(r['metrics']['capacity_blocked_packets'] for r in rows),
            'fed_offspring':sum(c['juvenile_received_milli']>0 for r in rows for c in r['funnel']['individuals'].values()),
            'maximum_death_age':max((c.get('death_age',0) for r in rows for c in r['funnel']['individuals'].values()),default=None),
            'life_cycle_certificates':summary['actual_life_cycle_certificates']})
    return {'completed_arms':len(result),'arms':result,
        'scope':'Only completed arms. Compare within each predeclared seed; population and regrowth are experimental settings. Raw counts are per declared horizon and can include natural restarts. Regrowth setting does not guarantee a proportional increase in available or consumed food. Proposal counts are packet-ticks rather than distinct partners. No claim of evolved closure without a certificate.'}


if __name__=='__main__':
    print(json.dumps(summarize_factorial(Path(sys.argv[1])),indent=2))
