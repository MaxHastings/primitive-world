//! Historical fraction gate for regression comparisons; production uses actual energy.
use super::*;

pub(super) fn install_legacy_fraction_gate(s: &mut Simulation, d: &wgpu::Device) {
    for (name, source, entry, old, new) in [
        (
            "interact_production",
            include_str!("../shaders/interactions.wgsl"),
            "production",
            "a.energy>=a.packet_size",
            "a.energy*d.amount>=a.packet_size",
        ),
        (
            "birth",
            include_str!("../shaders/apply_births.wgsl"),
            "main",
            "p.energy<p.packet_size",
            "p.energy*d.amount<p.packet_size",
        ),
    ] {
        assert_eq!(source.matches(old).count(), 1);
        let source = source.replace(old, new);
        let pass = s.passes.get_mut(name).unwrap();
        let layout = pass.pipeline.get_bind_group_layout(0);
        let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("legacy fraction gate"),
            source: wgpu::ShaderSource::Wgsl(shader_source(&source).into()),
        });
        let pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        pass.pipeline = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("legacy fraction gate"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            cache: None,
        });
    }
}

#[test]
fn production_gate_keeps_packet_cost_and_rejects_unaffordable_purchases() {
    let (d, q) = gpu();
    for (energy, production_count, experimental_count) in [(10.0, 0, 0), (35.0, 1, 0), (50.0, 1, 1)]
    {
        for (experimental, expected) in [(false, production_count), (true, experimental_count)] {
            let mut s = scene(&d, &q);
            if experimental {
                install_legacy_fraction_gate(&mut s, &d);
            }
            let mut a = body([100.0, 100.0]);
            a.energy = energy;
            a.food = 0.0;
            a.packet_size = 24.0;
            let mut g = fixed(5, [0.0; 2]);
            g[OUTPUT_BIAS + 8] = 0.0;
            put(&s, &q, 0, a, &g);
            step(&mut s, &d, &q, 1);
            let bodies = s.agent_snapshot(&d, &q).unwrap();
            let packets: Vec<_> = bodies.iter().filter(|b| b.alive == 2).collect();
            assert_eq!(packets.len(), expected);
            near(bodies[0].energy + bodies[0].spent, energy);
            for p in packets {
                near(p.energy, 24.0);
            }
            assert!(bodies[0].energy >= 0.0);
        }
    }
}
