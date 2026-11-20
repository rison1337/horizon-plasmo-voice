//! Bounded voxel traversal, evaluated on the game thread against loaded blocks.
pub fn transmission(
    from: [f64; 3],
    to: [f64; 3],
    mut absorption: impl FnMut([i32; 3]) -> f32,
) -> f32 {
    let delta: [f64; 3] = std::array::from_fn(|i| to[i] - from[i]);
    if delta.iter().any(|v| !v.is_finite()) {
        return 0.;
    }
    let mut cell = from.map(|v| v.floor() as i32);
    let end = to.map(|v| v.floor() as i32);
    let step = delta.map(|v| if v > 0. { 1 } else { -1 });
    let stride = delta.map(|v| {
        if v.abs() < 1e-10 {
            f64::INFINITY
        } else {
            1. / v.abs()
        }
    });
    let mut next: [f64; 3] = std::array::from_fn(|i| {
        if delta[i].abs() < 1e-10 {
            f64::INFINITY
        } else {
            (f64::from(cell[i]) + if step[i] > 0 { 1. } else { 0. } - from[i]) / delta[i]
        }
    });
    let mut gain = 1.;
    for _ in 0..256 {
        if cell == end {
            break;
        }
        let axis = (0..3).min_by(|&a, &b| next[a].total_cmp(&next[b])).unwrap();
        if next[axis] > 1. {
            break;
        }
        cell[axis] += step[axis];
        next[axis] += stride[axis];
        if cell == end {
            break;
        }
        let value = absorption(cell);
        if value.is_finite() {
            gain *= 1. - value.clamp(0., 0.98);
        }
        if gain < 0.01 {
            return 0.01;
        }
    }
    gain
}
