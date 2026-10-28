(function (global) {
  'use strict';
  function rows(root, folded = new Set()) {
    if (!root) return [];
    const out = [],
      width = (n) =>
        folded.has(n.id) || !n.children.length ? 1 : n.children.reduce((s, c) => s + width(c), 0);
    function walk(node, column, depth) {
      const span = width(node);
      out.push({ node, column, span, depth, y: -depth });
      if (!folded.has(node.id)) {
        let x = column;
        for (const child of node.children) {
          walk(child, x, depth + 1);
          x += width(child);
        }
      }
    }
    walk(root, 0, 0);
    return out;
  }
  function forNode(root, id, column, folded = new Set()) {
    const row = rows(root, folded).find((r) => r.node.id === id) || rows(root, folded)[0];
    return row
      ? {
          x: Math.max(
            row.column,
            Math.min(row.column + row.span - 1, Math.floor(column ?? row.column))
          ),
          y: row.y
        }
      : null;
  }
  function step(root, cursor, direction, folded = new Set()) {
    const all = rows(root, folded);
    if (!all.length) return null;
    cursor = cursor || forNode(root, root.id, 0, folded);
    return {
      x: Math.max(
        0,
        Math.min(
          all[0].span - 1,
          cursor.x + (direction === 'right' ? 1 : direction === 'left' ? -1 : 0)
        )
      ),
      y: Math.max(
        -Math.max(...all.map((r) => r.depth)),
        Math.min(0, cursor.y + (direction === 'up' ? -1 : direction === 'down' ? 1 : 0))
      )
    };
  }
  function target(root, cursor, folded = new Set()) {
    if (!cursor) return null;
    const all = rows(root, folded),
      r = all.find((r) => r.y === cursor.y && cursor.x >= r.column && cursor.x < r.column + r.span);
    if (!r) return null;
    if (!folded.has(r.node.id))
      for (let index = 0; index < r.node.children.length; index++) {
        const child = all.find((row) => row.node.id === r.node.children[index].id);
        if (cursor.x >= child.column && cursor.x < child.column + child.span)
          return {
            nodeId: r.node.id,
            index,
            childId: child.node.id,
            start: child.column,
            width: child.span
          };
      }
    return {
      nodeId: r.node.id,
      index: r.node.children.length,
      childId: null,
      start: r.column,
      width: r.span
    };
  }
  function destination(root, cursor, lift = false, folded = new Set()) {
    const t = target(root, cursor, folded);
    return t
      ? lift
        ? { placement: 'below', target: t.childId ?? t.nodeId }
        : { placement: 'input', target: t.nodeId, index: t.index }
      : null;
  }
  function slots(row, all) {
    const children = row.node.children
      .map((n) => all.find((r) => r.node.id === n.id))
      .filter(Boolean);
    if (!children.length)
      return [{ index: row.node.children.length, column: row.column + row.span / 2 }];
    return children
      .map((c, index) => ({ index, column: c.column }))
      .concat({ index: children.length, column: row.column + row.span });
  }
  function studs(row, all) {
    const children = row.node.children
      .map((n) => all.find((r) => r.node.id === n.id))
      .filter(Boolean);
    return children.length
      ? children.map((c) => c.column - row.column + c.span / 2)
      : [row.span / 2];
  }
  function geometry(root, folded = new Set(), { fractal = false, square = false } = {}) {
    const all = rows(root, folded);
    if (!all.length) return [];
    const byId = new Map(all.map((r) => [r.node.id, r]));
    if (!fractal && !square) {
      const depth = Math.max(...all.map((r) => r.depth));
      return all.map((r) => ({
        ...r,
        x: r.column,
        y: (depth - r.depth) * 112,
        width: r.span * 190 - 12,
        height: 112,
        scale: 1
      }));
    }
    function walk(node, x, bottom, scale) {
      const r = byId.get(node.id);
      r.x = x / 190;
      r.scale = scale;
      r.width = fractal ? 178 * scale : r.span * 190 - 12;
      r.height = square ? r.width : 112 * scale;
      r.y = bottom - r.height;
      const children = folded.has(node.id) ? [] : node.children;
      const childScale =
        fractal && children.length > 1 ? r.width / (children.length * 190 - 12) : scale;
      let left = x;
      for (const child of children) {
        walk(child, fractal ? left : byId.get(child.id).column * 190, r.y, childScale);
        left += 190 * childScale;
      }
    }
    walk(root, 0, 0, 1);
    const top = Math.min(...all.map((r) => r.y));
    for (const r of all) r.y -= top;
    return all;
  }
  function fit(all, { width, height }) {
    if (!all.length) return { scale: 1, pan: { x: 20, y: 40 } };
    const left = Math.min(...all.map((r) => r.x * 190)),
      right = Math.max(...all.map((r) => r.x * 190 + r.width)),
      top = Math.min(...all.map((r) => r.y - 10 * r.scale)),
      bottom = Math.max(...all.map((r) => r.y + r.height + 12 * r.scale));
    const w = Math.max(1, right - left),
      h = Math.max(1, bottom - top),
      availableWidth = Math.max(1, width - 40),
      availableHeight = Math.max(1, height - 105);
    const scale = Math.max(1e-6, Math.min(1.2, availableWidth / w, availableHeight / h));
    return {
      scale,
      pan: {
        x: 20 + (availableWidth - w * scale) / 2 - left * scale,
        y: 40 + (availableHeight - h * scale) / 2 - top * scale
      }
    };
  }
  function connections(row, all) {
    const children = row.node.children
      .map((n) => all.find((r) => r.node.id === n.id))
      .filter(Boolean);
    return children.length
      ? children.map((c) => ({ x: (c.x - row.x) * 190 + c.width / 2, scale: c.scale }))
      : [{ x: row.width / 2, scale: row.scale }];
  }
  function ports(row, all) {
    const children = row.node.children
        .map((n) => all.find((r) => r.node.id === n.id))
        .filter(Boolean),
      inset = Math.min(12 * row.scale, row.width / 2);
    return !children.length
      ? [{ index: row.node.children.length, x: row.width / 2 }]
      : children
          .map((c, index) => ({
            index,
            x: Math.max(inset, Math.min(row.width - inset, (c.x - row.x) * 190 - 6 * c.scale))
          }))
          .concat({ index: children.length, x: row.width - inset });
  }
  function outline(width, height, studs, socket, scale = 1) {
    const r = 12 * scale;
    let p = `M${r} 0`;
    for (const stud of studs) {
      const c = typeof stud === 'number' ? stud : stud.x,
        s = typeof stud === 'number' ? 1 : stud.scale;
      p += ` H${c - 16 * s} Q${c - 12 * s} 0 ${c - 12 * s} ${-4 * s} V${-7 * s} Q${c - 12 * s} ${-10 * s} ${c - 8 * s} ${-10 * s} H${c + 8 * s} Q${c + 12 * s} ${-10 * s} ${c + 12 * s} ${-7 * s} V${-4 * s} Q${c + 12 * s} 0 ${c + 16 * s} 0`;
    }
    const c = socket,
      s = scale;
    p += ` H${width - r} Q${width} 0 ${width} ${r} V${height - r} Q${width} ${height} ${width - r} ${height} H${c + 16 * s} Q${c + 12 * s} ${height} ${c + 12 * s} ${height - 4 * s} V${height - 7 * s} Q${c + 12 * s} ${height - 10 * s} ${c + 8 * s} ${height - 10 * s} H${c - 8 * s} Q${c - 12 * s} ${height - 10 * s} ${c - 12 * s} ${height - 7 * s} V${height - 4 * s} Q${c - 12 * s} ${height} ${c - 16 * s} ${height} H${r} Q0 ${height} 0 ${height - r} V${r} Q0 0 ${r} 0 Z`;
    return p;
  }
  function underside(cards, point, scale = 1) {
    return (
      cards
        .filter(
          (r) =>
            point.x >= r.left &&
            point.x <= r.right &&
            Math.abs(point.y - r.bottom) <= 14 * scale * (r.scale ?? 1)
        )
        .sort((a, b) => Math.abs(point.y - a.bottom) - Math.abs(point.y - b.bottom))[0]?.id ?? null
    );
  }
  const api = {
    geometry,
    fit,
    connections,
    ports,
    rows,
    forNode,
    step,
    target,
    destination,
    slots,
    studs,
    outline,
    underside
  };
  global.SchemaStackLayout = api;
  if (typeof module !== 'undefined') module.exports = api;
})(globalThis);
