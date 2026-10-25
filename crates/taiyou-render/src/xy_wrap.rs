//! Independently expressed line policy observed in the pinned local pinterf RTS
//! GetFullLineWidth/GetWrapWidth/GetNextLine/Compact paths. Widths are backend
//! inputs, not GDI or authenticated xy capture results. No Unicode break rules.
use taiyou_types::{ErrorCode, RenderError};

#[derive(Clone, Debug)]
pub(crate) struct Word {
    pub start: u32,
    pub end: u32,
    pub width: i64,
    pub space: bool,
    pub hard: bool,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Span {
    pub start: u32,
    pub end: u32,
    pub visible_start: u32,
    pub visible_end: u32,
}
fn limit() -> RenderError {
    RenderError::new(
        ErrorCode::LimitExceeded,
        "xy wrapping width/allocation overflow",
    )
}
fn add(a: i64, b: i64) -> Result<i64, RenderError> {
    a.checked_add(b).ok_or_else(limit)
}
// GetTextExtentPoint32 sums integer glyph extents. Native glyph metrics first
// pass through the 28.4 device grid; rounding the continuous word sum instead
// moves an entire centred line at native half-grid thresholds. Ink advances
// remain separate and continuous, as in TextOut's path construction.
pub(crate) fn measured_advance(advance: f64, grid: f64) -> Result<f64, RenderError> {
    let fixed = (advance * grid * 1024.0 + 0.5).floor();
    if !fixed.is_finite() || fixed.abs() >= i64::MAX as f64 || grid <= 0.0 {
        return Err(limit());
    }
    Ok(((fixed + 8.0) / 16.0).floor() / (64.0 * grid))
}
// xy's width storage is eighth-pixels; its font extent has 64 units/pixel.
pub(crate) fn width_units(advance: f64) -> Result<i64, RenderError> {
    let value = (advance * 64.0 + 4.0).trunc();
    if !value.is_finite() || value.abs() >= i64::MAX as f64 {
        return Err(limit());
    }
    Ok((value as i64).div_euclid(8))
}
pub(crate) fn lines(
    words: &[Word],
    end: u32,
    mode: u8,
    available: i64,
) -> Result<Vec<Span>, RenderError> {
    let mut suffix = Vec::new();
    let count = words.len().checked_add(1).ok_or_else(limit)?;
    suffix.try_reserve_exact(count).map_err(|_| limit())?;
    suffix.resize(count, 0_i64);
    for i in (0..words.len()).rev() {
        suffix[i] = if words[i].hard {
            0
        } else {
            add(words[i].width, suffix[i + 1])?
        };
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(words.len().max(1))
        .map_err(|_| limit())?;
    let mut cursor = 0;
    while cursor < words.len() {
        let begin = cursor;
        let mut capacity = if mode == 2 { i64::MAX } else { available };
        if matches!(mode, 0 | 3) && available > 0 {
            let full = suffix[cursor];
            let denominator = add(full.checked_abs().ok_or_else(limit)? / available, 1)?;
            let target = full / denominator;
            let (mut accumulated, mut last) = (0_i64, 0_i64);
            for word in &words[cursor..] {
                if accumulated >= target {
                    break;
                }
                last = word.width;
                let next = add(accumulated, last)?;
                if next.checked_abs().ok_or_else(limit)? < available {
                    accumulated = next;
                }
            }
            if mode == 3
                && accumulated < full
                && add(full.checked_sub(accumulated).ok_or_else(limit)?, last)? < available
            {
                accumulated = accumulated.checked_sub(last).ok_or_else(limit)?;
            }
            capacity = accumulated;
        }
        let mut width = 0;
        let mut hard = false;
        while cursor < words.len() {
            if words[cursor].hard {
                hard = true;
                break;
            }
            let space = words[cursor].space;
            let group = cursor;
            let mut group_width = 0;
            while cursor < words.len() && !words[cursor].hard && words[cursor].space == space {
                group_width = add(group_width, words[cursor].width)?;
                cursor += 1;
            }
            let next = add(width, group_width)?;
            if next <= capacity || group == begin {
                width = next;
            } else {
                cursor = group;
                break;
            }
        }
        let line_end = if hard {
            words[cursor].start
        } else {
            words.get(cursor).map_or(end, |w| w.start)
        };
        let (mut left, mut right) = (begin, cursor);
        while left < right && words[left].space {
            left += 1;
        }
        while left < right && words[right - 1].space {
            right -= 1;
        }
        let visible_start = words.get(left).map_or(line_end, |w| w.start).min(line_end);
        let visible_end = if left == right {
            visible_start
        } else {
            words[right - 1].end
        };
        result.push(Span {
            start: words[begin].start,
            end: line_end,
            visible_start,
            visible_end,
        });
        if hard {
            cursor += 1;
        }
    }
    if result.is_empty() {
        result.push(Span {
            start: 0,
            end,
            visible_start: 0,
            visible_end: 0,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_word_measurement_rounds_glyph_extents_before_summing() {
        // Measured GDI DejaVu Sans advances at native EM1649 and3299. At the
        // smaller size 'a' crosses an integer tie only through the 28.4 grid.
        let a = super::measured_advance(1255.0 * 1649.0 / 2048.0 / 64.0, 1.0).unwrap();
        assert_eq!(a * 64.0, 1011.0);
        let measured = [1300.0, 1260.0]
            .into_iter()
            .map(|units| super::measured_advance(units * 3299.0 / 2048.0 / 64.0, 1.0).unwrap())
            .sum::<f64>();
        assert_eq!(measured * 64.0, 4124.0);
        assert_eq!(super::width_units(measured).unwrap(), 516);
        assert_eq!(
            super::width_units(2560.0 * 3299.0 / 2048.0 / 64.0).unwrap(),
            515
        );
        assert!(super::measured_advance(f64::INFINITY, 1.0).is_err());
    }
    use super::*;
    fn words(widths: &[i64]) -> Vec<Word> {
        widths
            .iter()
            .enumerate()
            .map(|(i, &width)| Word {
                start: i as u32,
                end: i as u32 + 1,
                width,
                space: i % 2 == 1,
                hard: false,
            })
            .collect()
    }
    #[test]
    fn smart_alternative_and_greedy_have_source_policy_decisions() {
        let input = words(&[40, 10, 40, 10, 40]);
        let decisions = |q| {
            lines(&input, 5, q, 100)
                .unwrap()
                .iter()
                .map(|l| (l.visible_start, l.visible_end))
                .collect::<Vec<_>>()
        };
        assert_eq!(decisions(0), [(0, 3), (4, 5)]);
        assert_eq!(decisions(1), [(0, 3), (4, 5)]);
        assert_eq!(decisions(2), [(0, 5)]);
        assert_eq!(decisions(3), [(0, 1), (2, 5)]);
    }
    #[test]
    fn overwide_first_groups_negative_width_and_compaction_make_progress() {
        let input = words(&[200, 10, 30]);
        for q in [0, 1, 3] {
            let out = lines(&input, 3, q, 100).unwrap();
            assert_eq!((out[0].visible_start, out[0].visible_end), (0, 1));
            assert_eq!(out.last().unwrap().visible_end, 3);
            assert!(lines(&input, 3, q, -1).unwrap().len() <= input.len());
        }
        assert!(width_units(f64::INFINITY).is_err());
        assert_eq!(width_units(1.0625).unwrap(), 9);
    }
}
