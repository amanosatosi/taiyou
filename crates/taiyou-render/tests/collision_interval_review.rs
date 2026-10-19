use std::{io::Write, time::Instant};
use taiyou_render::collision::*;
use taiyou_types::*;

// Independent preserved pre-review implementation, used only as a small-input
// oracle and bounded experiment. It is never part of the production renderer.
mod previous_collision;
use previous_collision as previous;

fn f(value: f64) -> FiniteF64 {
    FiniteF64::new(value).unwrap()
}
fn input(id: u32) -> CollisionInput {
    CollisionInput {
        id: EventId::new(id),
        layer: 0,
        start: TimeUs::from_micros(0),
        alignment: Alignment::new(2).unwrap(),
        anchor: ScriptPoint::new(f(0.0), f(0.0)),
        bounds: ScriptBounds::new(0.0, 0.0, 1.0, 1.0).unwrap(),
        eligible: true,
    }
}
fn old(events: &[CollisionInput]) -> Vec<previous::PlacedEvent> {
    let input: Vec<_> = events
        .iter()
        .map(|e| previous::CollisionInput {
            id: e.id,
            layer: e.layer,
            start: e.start,
            alignment: e.alignment,
            anchor: e.anchor,
            bounds: e.bounds,
            eligible: e.eligible,
        })
        .collect();
    previous::resolve_collisions(&input, Profile::CoreV1, &Limits::default()).unwrap()
}
fn next(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed
}

#[test]
fn randomized_fractional_mixed_layer_order_matches_independent_previous_solver() {
    let mut seed = 982_451_653;
    for _ in 0..1_000 {
        let count = next(&mut seed) % 30;
        let mut events = Vec::new();
        for id in 0..count {
            let mut event = input(id);
            event.layer = (next(&mut seed) % 5) as i32 - 2;
            event.start = TimeUs::from_micros(i64::from(next(&mut seed) % 4));
            event.alignment = Alignment::new((next(&mut seed) % 9 + 1) as u8).unwrap();
            event.eligible = next(&mut seed) % 4 != 0;
            event.anchor = ScriptPoint::new(
                f(f64::from(next(&mut seed) % 101) / 10.0 - 5.0),
                f(f64::from(next(&mut seed) % 101) / 16.0 - 3.0),
            );
            let x = f64::from(next(&mut seed) % 17) / 4.0;
            let y = f64::from(next(&mut seed) % 17) / 8.0;
            event.bounds = ScriptBounds::new(
                -x,
                -y,
                x + f64::from(next(&mut seed) % 5),
                y + f64::from(next(&mut seed) % 5),
            )
            .unwrap();
            events.push(event);
        }
        for i in (1..events.len()).rev() {
            let j = next(&mut seed) as usize % (i + 1);
            events.swap(i, j);
        }
        let expected = old(&events);
        let actual = resolve_collisions(&events, Profile::CoreV1, &Limits::default()).unwrap();
        assert_eq!(actual.len(), expected.len());
        for (a, b) in actual.iter().zip(expected) {
            assert_eq!((a.id, a.anchor, a.bounds), (b.id, b.anchor, b.bounds));
        }
    }
}

#[test]
fn open_interval_components_preserve_legal_touching_anchor() {
    let mut a = input(0);
    let mut b = input(1);
    let mut c = input(2);
    a.bounds = ScriptBounds::new(0.0, -2.0, 1.0, -1.0).unwrap();
    b.bounds = ScriptBounds::new(0.0, 1.0, 1.0, 2.0).unwrap();
    c.bounds = ScriptBounds::new(0.0, -1.0, 1.0, 1.0).unwrap();
    let actual = resolve_collisions(&[a, b, c], Profile::CoreV1, &Limits::default()).unwrap();
    assert_eq!(actual[2].anchor.y().get(), 0.0);
}

#[test]
fn five_thousand_explicit_events_do_not_add_collision_work() {
    let automatic = [input(10_000), input(10_001)];
    let (expected, stats) =
        resolve_collisions_with_stats(&automatic, Profile::CoreV1, &Limits::default()).unwrap();
    let mut mixed = Vec::new();
    for id in 0..5_000 {
        let mut event = input(id);
        event.eligible = false;
        mixed.push(event);
    }
    let (_, empty) =
        resolve_collisions_with_stats(&mixed, Profile::CoreV1, &Limits::default()).unwrap();
    assert_eq!(
        (
            empty.eligible,
            empty.obstacle_tests,
            empty.intervals,
            empty.sort_comparisons,
            empty.interval_visits
        ),
        (0, 0, 0, 0, 0)
    );
    mixed.extend(automatic);
    mixed.reverse();
    let (actual, mixed_stats) =
        resolve_collisions_with_stats(&mixed, Profile::CoreV1, &Limits::default()).unwrap();
    assert_eq!(&actual[5_000..], expected);
    assert_eq!(
        (
            mixed_stats.eligible,
            mixed_stats.obstacle_tests,
            mixed_stats.intervals,
            mixed_stats.sort_comparisons,
            mixed_stats.interval_visits
        ),
        (
            stats.eligible,
            stats.obstacle_tests,
            stats.intervals,
            stats.sort_comparisons,
            stats.interval_visits
        )
    );
}

#[test]
fn scaling_measurement_on_actions_native_linux() {
    let Ok(folder) = std::env::var("TAIYOU_RENDER_SAMPLE_DIR") else {
        return;
    };
    if !cfg!(target_os = "linux")
        || std::env::var("RUSTUP_TOOLCHAIN").unwrap_or_default() != "1.99.1"
    {
        return;
    }
    let mut lines = String::new();
    for count in [100_u32, 500, 1_000, 5_000] {
        let events: Vec<_> = (0..count).map(input).collect();
        let old_operations = u64::from(count) * u64::from(count - 1) * u64::from(count + 4) / 6;
        // The old 5000-event run exceeds the explicit experiment work budget
        // (20 billion checks). Do not spend the runner's timeout proving that.
        let old_ns = if old_operations <= 200_000_000 {
            let start = Instant::now();
            let result = old(&events);
            assert_eq!(
                result.last().unwrap().anchor.y().get(),
                -f64::from(count - 1)
            );
            Some(start.elapsed().as_nanos())
        } else {
            None
        };
        for sample in 0..3 {
            let start = Instant::now();
            let (actual, stats) =
                resolve_collisions_with_stats(&events, Profile::CoreV1, &Limits::default())
                    .unwrap();
            let elapsed = start.elapsed().as_nanos();
            assert_eq!(
                actual.last().unwrap().anchor.y().get(),
                -f64::from(count - 1)
            );
            assert_eq!(
                stats.obstacle_tests,
                u64::from(count) * u64::from(count - 1) / 2
            );
            lines.push_str(&format!("{{\"events\":{count},\"sample\":{sample},\"new_ns\":{elapsed},\"old_ns\":{},\"old_obstacle_checks_derived\":{old_operations},\"new_obstacle_checks\":{},\"intervals\":{},\"sort_comparisons\":{},\"interval_visits\":{},\"reserved_workspace_bytes\":{},\"old_workspace_bytes\":{}}}\n", old_ns.map_or_else(|| "null".to_owned(), |n| n.to_string()), stats.obstacle_tests, stats.intervals, stats.sort_comparisons, stats.interval_visits, stats.reserved_bytes, u64::from(count) * 3 * (std::mem::size_of::<usize>() + std::mem::size_of::<previous::PlacedEvent>()) as u64));
        }
    }
    std::fs::create_dir_all(&folder).unwrap();
    for line in lines.lines() {
        let record = format!("T017_SCALE_SAMPLE={line}\n");
        std::io::stdout().write_all(record.as_bytes()).unwrap();
    }
    std::fs::write(
        std::path::Path::new(&folder).join("T017-collision-scaling.jsonl"),
        lines,
    )
    .unwrap();
}
