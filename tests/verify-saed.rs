// 直接编译 NRO 使用的纯逻辑，核对独立时间／位置预期及真实原版、asset HDP。
#[path = "../Moveset-to-4G/src/hitdata/records.rs"]
mod hitdata;
#[path = "../Moveset-to-4G/src/saed/schedule.rs"]
mod schedule;
#[path = "../shared/hitdata_path.rs"] mod cb_path;

use schedule::{PhialKind, Step};

const ELEMENT: [(f32, f32, f32); 10] = [
    (0.0, 0.0, 300.0),
    (18.0, 80.0, 420.0),
    (20.0, -80.0, 420.0),
    (38.0, -120.0, 540.0),
    (40.0, 120.0, 540.0),
    (42.0, 0.0, 540.0),
    (58.0, -160.0, 660.0),
    (60.0, 160.0, 660.0),
    (62.0, -160.0 / 3.0, 660.0),
    (64.0, 160.0 / 3.0, 660.0),
];

fn timing() {
    for kind in [PhialKind::Impact, PhialKind::Element] {
        for total in 1..=10 {
            for delta in [0.25f32, 0.5, 1.0, 2.5, 20.0] {
                let mut sequence = schedule::Sequence::new(kind, total);
                let mut previous = 0.0f32;
                let mut first = 0.0f32;
                let mut index = 0usize;
                let mut stopped = false;
                for tick in 1..=2000 {
                    let now = tick as f32 * delta;
                    match sequence.advance(delta) {
                        Step::Blast {
                            index: actual,
                            remaining,
                        } => {
                            assert_eq!(actual as usize, index);
                            assert_eq!(remaining, total - actual - 1);
                            let at = if kind == PhialKind::Impact {
                                index as f32 * 9.0
                            } else {
                                ELEMENT[index].0
                            };
                            let delay = if kind == PhialKind::Impact {
                                0.0f32
                            } else {
                                46.0
                            };
                            let due = if index == 0 {
                                (delay / delta).ceil() * delta
                            } else {
                                first + (at / delta).ceil() * delta
                            };
                            let due = due.max(previous + delta);
                            assert_eq!(now, due, "计时漂移或重复瓶爆");
                            previous = now;
                            if index == 0 {
                                first = now;
                            }
                            index += 1;
                        }
                        Step::Stop => {
                            assert_eq!(index, total as usize);
                            assert!(now >= previous + 9.0 && now - delta < previous + 9.0);
                            stopped = true;
                            break;
                        }
                        Step::Wait => {}
                    }
                }
                assert!(stopped, "最后一爆未回收");
            }
        }
    }
    // 暂停不提前消耗计时；两位玩家的进程互不影响。
    let mut first = schedule::Sequence::new(PhialKind::Impact, 2);
    let mut second = schedule::Sequence::new(PhialKind::Element, 10);
    for _ in 0..100 {
        assert_eq!(first.advance(0.0), Step::Wait);
    }
    assert_eq!(
        first.advance(1.0),
        Step::Blast {
            index: 0,
            remaining: 1
        }
    );
    assert_eq!(second.advance(5.0), Step::Wait);
    assert_eq!(first.advance(8.0), Step::Wait);
    assert_eq!(
        first.advance(1.0),
        Step::Blast {
            index: 1,
            remaining: 0
        }
    );
    assert_eq!(
        second.advance(41.0),
        Step::Blast {
            index: 0,
            remaining: 9
        }
    );
}

fn position() {
    for index in 0..10 {
        let impact = schedule::blast(PhialKind::Impact, index);
        assert_eq!(
            (impact.at, impact.side, impact.forward),
            (index as f32 * 9.0, 0.0, 270.0 + index as f32 * 100.0)
        );
        let element = schedule::blast(PhialKind::Element, index);
        assert_eq!(
            (element.at, element.side, element.forward),
            ELEMENT[index as usize]
        );
        for degrees in (0..360).step_by(5) {
            let yaw = (degrees as f32).to_radians();
            let heading = [yaw.sin(), yaw.cos()];
            let origin = [120.0, 35.0, -680.0];
            let world = schedule::world_xz(origin, heading, element);
            assert_eq!(world[1], origin[1]);
            let offset = [world[0] - origin[0], world[2] - origin[2]];
            assert!((offset[0] * heading[1] - offset[1] * heading[0] - element.side).abs() < 0.001);
            assert!(
                (offset[0] * heading[0] + offset[1] * heading[1] - element.forward).abs() < 0.001
            );
        }
    }
    for (start, end, row_width) in [(1, 3, 80.0), (3, 6, 120.0), (6, 10, 160.0)] {
        assert!((ELEMENT[start..end].iter().map(|v| v.1).sum::<f32>()).abs() < 0.001);
        assert_eq!(
            ELEMENT[start..end]
                .iter()
                .map(|v| v.1.abs())
                .fold(0.0, f32::max),
            row_width
        );
    }
}

fn hdp(original: &[u8], asset: &[u8], new_hit: &[u8]) {
    assert_eq!(original.len(), 4496);
    for input in [original, asset] {
        let mut extended = vec![0xCD; hitdata::EXTENDED_SIZE];
        hitdata::extend(input, &mut extended);
        assert_eq!(extended.len(), 4552);
        assert_eq!(u32::from_le_bytes(extended[8..12].try_into().unwrap()), 81);
        assert_eq!(&extended[..8], &input[..8]);
        assert_eq!(&extended[12..16], &input[12..16]);
        assert_eq!(&extended[16..4496], &input[16..4496], "原有 HDP 条目遭修改");
        assert_eq!(&extended[4496..], new_hit, "0x50 与第三下基准快照不同");
        assert_eq!(
            u16::from_le_bytes(extended[4500..4502].try_into().unwrap()),
            100
        );
    }
    for path in [
        b"player\\hit\\pl_we14_hitdata".as_slice(),
        b"HIT/PL_WE14_HITDATA.hdp",
        b"player/hit/pl_we14_hitdata\0ignored",
    ] {
        assert!(cb_path::is_cb_path(path));
    }
    for path in [
        b"player\\hit\\pl_we13_hitdata".as_slice(),
        b"player\\hit\\pl_we14_hitdata_other",
        b"pl_we14_hitdata",
    ] {
        assert!(!cb_path::is_cb_path(path));
    }
}

fn native_fan_positions(bytes: &[u8]) {
    let mut maximum_error = 0.0f32;
    for record in bytes.chunks_exact(32) {
        let index = u32::from_le_bytes(record[..4].try_into().unwrap()) as u8;
        let float_at = |offset| f32::from_le_bytes(record[offset..offset + 4].try_into().unwrap());
        let heading = [float_at(4), float_at(8)];
        let origin = [float_at(12), float_at(16), float_at(20)];
        let actual =
            schedule::world_xz(origin, heading, schedule::blast(PhialKind::Element, index));
        let error = (actual[0] - float_at(24))
            .abs()
            .max((actual[2] - float_at(28)).abs());
        maximum_error = maximum_error.max(error);
        assert!(
            error < 0.001,
            "第 {} 爆左右旋转与 4G 原指令不一致：误差 {}",
            index + 1,
            error
        );
    }
    assert_eq!(bytes.len(), 1536 * 32);
    println!(
        "PASS: 1536 element positions match original 4G ARM rotation; max error {maximum_error}"
    );
}

fn main() {
    let arguments: Vec<_> = std::env::args().collect();
    timing();
    position();
    native_fan_positions(&std::fs::read(&arguments[4]).unwrap());
    hdp(
        &std::fs::read(&arguments[1]).unwrap(),
        &std::fs::read(&arguments[2]).unwrap(),
        &std::fs::read(&arguments[3]).unwrap(),
    );
    println!(
        "PASS: 100 sequences (1-10 phials, both types, five tick sizes), pause/isolation, 720 rotations, partial fan rows; original/asset HDP expanded to 81 records, original records preserved and 50 matches the captured third-hit fixture"
    );
}
