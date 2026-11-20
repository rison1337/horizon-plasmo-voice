// Original 16x16 pixel art. Every microphone shares the same centred capsule,
// cradle and baseline; state marks never move or resize that silhouette.
#[derive(Clone, Copy)]
pub(super) enum Glyph {
    Microphone,
    Speaker,
    Headphones,
    Reset,
}
#[derive(Clone, Copy)]
pub(super) enum Mark {
    None,
    Talking(u8),
    Muted,
    Locked,
    Connecting(u8),
    Missing,
}
pub(super) type Pixels = [[u32; 16]; 16];
const OUTLINE: u32 = 0xff20282b;
const GREEN: u32 = 0xff79e38d;
const RED: u32 = 0xffff6262;
const GOLD: u32 = 0xffffcd69;

fn rect(p: &mut Pixels, x: usize, y: usize, w: usize, h: usize, c: u32) {
    for row in p.iter_mut().skip(y).take(h) {
        for cell in row.iter_mut().skip(x).take(w) {
            *cell = c;
        }
    }
}
fn shade(c: u32, factor: u32) -> u32 {
    0xff000000
        | (((c >> 16 & 255) * factor / 255) << 16)
        | (((c >> 8 & 255) * factor / 255) << 8)
        | ((c & 255) * factor / 255)
}
pub(super) fn sprite(glyph: Glyph, tint: u32, mark: Mark) -> Pixels {
    let mut p = [[0; 16]; 16];
    let light = 0xff000000 | tint;
    let dark = shade(light, 190);
    match glyph {
        Glyph::Microphone => {
            // A rounded capsule, a separate U-shaped cradle and a flat foot.
            // The two centre columns are x=7/8 in every activation state.
            let capsule = if matches!(mark, Mark::Talking(_)) {
                GREEN
            } else {
                light
            };
            rect(&mut p, 7, 2, 2, 1, capsule);
            rect(&mut p, 6, 3, 4, 5, capsule);
            rect(&mut p, 9, 4, 1, 4, shade(capsule, 200));
            rect(&mut p, 7, 8, 2, 1, shade(capsule, 200));
            rect(&mut p, 4, 5, 1, 4, light);
            rect(&mut p, 11, 5, 1, 4, light);
            rect(&mut p, 5, 9, 1, 1, light);
            rect(&mut p, 10, 9, 1, 1, light);
            rect(&mut p, 6, 10, 4, 1, light);
            rect(&mut p, 7, 11, 2, 2, dark);
            rect(&mut p, 5, 13, 6, 1, light);
        }
        Glyph::Speaker => {
            rect(&mut p, 3, 6, 3, 4, light);
            rect(&mut p, 5, 5, 2, 6, light);
            rect(&mut p, 6, 4, 2, 8, light);
            rect(&mut p, 7, 4, 1, 8, dark);
        }
        Glyph::Headphones => {
            rect(&mut p, 5, 3, 6, 1, light);
            rect(&mut p, 4, 4, 1, 1, light);
            rect(&mut p, 11, 4, 1, 1, light);
            rect(&mut p, 3, 5, 1, 6, light);
            rect(&mut p, 12, 5, 1, 6, light);
            rect(&mut p, 4, 8, 2, 5, light);
            rect(&mut p, 10, 8, 2, 5, light);
            rect(&mut p, 5, 9, 1, 3, dark);
            rect(&mut p, 10, 9, 1, 3, dark);
        }
        Glyph::Reset => {
            // Counter-clockwise loop and an unambiguous stepped arrow head.
            for (x, y, w, h) in [
                (5, 2, 5, 2),
                (3, 3, 3, 2),
                (2, 5, 2, 2),
                (10, 3, 2, 2),
                (11, 5, 2, 6),
                (9, 11, 3, 2),
                (5, 12, 4, 2),
                (3, 10, 2, 3),
                (0, 6, 6, 1),
                (1, 7, 4, 1),
                (2, 8, 2, 1),
            ] {
                rect(&mut p, x, y, w, h, light);
            }
        }
    }
    if matches!(glyph, Glyph::Speaker) && matches!(mark, Mark::None) {
        rect(&mut p, 10, 6, 1, 4, light);
        rect(&mut p, 13, 4, 1, 8, dark);
    }
    if let Mark::Talking(phase) = mark {
        let height = [2, 4, 6, 4][usize::from(phase % 4)];
        if matches!(glyph, Glyph::Microphone) {
            // Balanced side bars leave the capsule fixed at the head anchor.
            rect(&mut p, 1, 8 - height / 2, 1, height, GREEN);
            rect(&mut p, 14, 8 - height / 2, 1, height, GREEN);
        } else {
            rect(&mut p, 10, 8 - height / 2, 1, height, GREEN);
            rect(&mut p, 13, 7 - height / 2, 1, height + 2, GREEN);
        }
    }
    if matches!(mark, Mark::Muted) {
        // A narrow connected staircase, inset from the border. The capsule,
        // cradle and foot remain recognisable behind the mute stroke.
        for y in 3..13 {
            p[y][14 - y] = OUTLINE;
        }
        for y in 3..13 {
            p[y][15 - y] = RED;
            if y > 3 {
                p[y][16 - y] = RED;
            }
        }
    }
    if matches!(mark, Mark::Locked) {
        rect(&mut p, 10, 8, 5, 6, OUTLINE);
        rect(&mut p, 12, 8, 1, 1, GOLD);
        rect(&mut p, 11, 9, 1, 1, GOLD);
        rect(&mut p, 13, 9, 1, 1, GOLD);
        rect(&mut p, 11, 10, 3, 3, GOLD);
        p[11][12] = OUTLINE;
    }
    if let Mark::Connecting(phase) = mark {
        rect(&mut p, 5, 12, 9, 3, OUTLINE);
        for i in 0..3 {
            p[13][6 + i * 3] = if i == usize::from(phase % 3) {
                GOLD
            } else {
                dark
            };
        }
    }
    if matches!(mark, Mark::Missing) {
        rect(&mut p, 7, 5, 3, 1, GOLD);
        rect(&mut p, 9, 6, 1, 2, GOLD);
        rect(&mut p, 8, 8, 1, 2, GOLD);
        p[11][8] = GOLD;
    }
    let mut outlined = p;
    for y in 0..16 {
        for x in 0..16 {
            if p[y][x] == 0 {
                // One-pixel contour without square corner dilation: separate
                // microphone parts keep their negative space at small scales.
                let colored = |c| c != 0 && c != OUTLINE;
                let neighbor = (x > 0 && colored(p[y][x - 1]))
                    || (x < 15 && colored(p[y][x + 1]))
                    || (y > 0 && colored(p[y - 1][x]))
                    || (y < 15 && colored(p[y + 1][x]));
                if neighbor {
                    outlined[y][x] = OUTLINE;
                }
            }
        }
    }
    outlined
}

pub(super) fn phase() -> u8 {
    use std::{sync::OnceLock, time::Instant};
    static START: OnceLock<Instant> = OnceLock::new();
    (START.get_or_init(Instant::now).elapsed().as_millis() / 180 % 12) as u8
}
