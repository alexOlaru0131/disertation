// Metri, conform software/raspi/src/legs.rs. Unghiurile STATE sunt în grade.
export const LENGTHS = [0.05, 0.1, 0.1];
export const DEG = Math.PI / 180;
export const add = (a, b) => a.map((v, i) => v + b[i]);
export const subtract = (a, b) => a.map((v, i) => v - b[i]);
export const dot = (a, b) => a.reduce((sum, v, i) => sum + v * b[i], 0);
export const norm = (v) => Math.hypot(...v);

export function parseState(values) {
  if (!Array.isArray(values) || values.length !== 84 || !values.every(Number.isFinite)) return null;
  const legs = Array.from({ length: 6 }, (_, index) => {
    const offset = 6 + index * 13;
    return {
      foot: values.slice(offset, offset + 3),
      angles: values.slice(offset + 3, offset + 6),
      motor: values.slice(offset + 6, offset + 9),
      motorRotation: values.slice(offset + 9, offset + 12),
      centerToMotor: values[offset + 12],
    };
  });
  return { body: { position: values.slice(0, 3), rotation: values.slice(3, 6) }, legs };
}

// Rz(yaw) * Ry(pitch) * Rx(roll). Matricea este calculată o dată per transformare.
export function transform(position, rotation) {
  const [r, p, y] = rotation.map((v) => v * DEG);
  const [sr, cr, sp, cp, sy, cy] = [Math.sin(r), Math.cos(r), Math.sin(p), Math.cos(p), Math.sin(y), Math.cos(y)];
  const rows = [
    [cy * cp, cy * sp * sr - sy * cr, cy * sp * cr + sy * sr],
    [sy * cp, sy * sp * sr + cy * cr, sy * sp * cr - cy * sr],
    [-sp, cp * sr, cp * cr],
  ];
  return {
    toWorld: (point) => rows.map((row, i) => dot(row, point) + position[i]),
    toLocal(point) {
      const offset = subtract(point, position);
      return [0, 1, 2].map((i) => rows.reduce((sum, row, j) => sum + row[i] * offset[j], 0));
    },
  };
}

// Articulațiile piciorului în sistemul local al corpului.
export function legJoints(leg) {
  const [l1, l2, l3] = LENGTHS;
  const [a, b, c] = leg.angles.map((v) => v * DEG);
  const radial = (r, z) => [r * Math.cos(a), r * Math.sin(a), z];
  const mount = transform(leg.motor, leg.motorRotation);
  return [
    [0, 0, 0],
    radial(l1, 0),
    radial(l1 + l2 * Math.cos(b), l2 * Math.sin(b)),
    radial(l1 + l2 * Math.cos(b) + l3 * Math.cos(b + c), l2 * Math.sin(b) + l3 * Math.sin(b + c)),
  ].map(mount.toWorld);
}

const wrap = (angle) => Math.atan2(Math.sin(angle), Math.cos(angle));

// Folosit doar în editor: păstrează lungimile la tragerea unui vârf.
// Dintre soluțiile posibile se alege cea mai apropiată de postura curentă.
export function anglesForFoot(leg, foot) {
  const [l1, l2, l3] = LENGTHS;
  const [x, y, z] = transform(leg.motor, leg.motorRotation).toLocal(foot);
  const horizontal = Math.hypot(x, y);
  const current = leg.angles.map((v) => v * DEG);
  const yaw = horizontal < 1e-10 ? current[0] : Math.atan2(y, x);
  let best = null;
  let bestDistance = Infinity;

  for (const direction of [1, -1]) {
    const radius = direction * horizontal - l1;
    const cosine = (radius * radius + z * z - l2 * l2 - l3 * l3) / (2 * l2 * l3);
    if (!Number.isFinite(cosine) || Math.abs(cosine) > 1 + 1e-10) continue;
    for (const sign of [-1, 1]) {
      const knee = sign * Math.acos(Math.max(-1, Math.min(1, cosine)));
      const femur = Math.hypot(radius, z) < 1e-10 && l2 === l3
        ? current[1]
        : Math.atan2(z, radius) - Math.atan2(l3 * Math.sin(knee), l2 + l3 * Math.cos(knee));
      const angles = [yaw + (direction < 0 ? Math.PI : 0), femur, knee].map(wrap);
      const distance = angles.reduce((sum, a, i) => sum + wrap(a - current[i]) ** 2, 0);
      if (distance < bestDistance) {
        best = angles.map((v) => v / DEG);
        bestDistance = distance;
      }
    }
  }
  if (!best) throw new Error("Vârful este în afara razei piciorului. Ultima poziție validă a fost păstrată.");
  return best;
}
