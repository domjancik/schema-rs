/* Fixture-anchored authoring overlays; camera/UI preferences never enter the project. */
(function (g) {
  'use strict';
  const fixtures = (p, mode, selected) =>
    mode === 'off' ? [] : p.fixtures.filter((f) => mode === 'all' || selected.has(f.id));
  function count(root) {
    return root ? 1 + root.children.reduce((s, n) => s + count(n), 0) : 0;
  }
  function pose(rows, point, scale) {
    const root = rows[0];
    return {
      x: point.x - (root ? root.x * 190 + root.width / 2 : 89) * scale,
      y: point.y - 18 - (root ? root.y + root.height : 0) * scale,
      scale
    };
  }
  function mount(body, controls, options) {
    const layer = document.createElement('div');
    layer.className = 'scene-stacks';
    body.append(layer);
    const select = document.createElement('select');
    select.setAttribute('aria-label', 'Stacks in scene');
    for (const [value, label] of [
      ['off', 'Stacks off'],
      ['selected', 'Selected stacks'],
      ['all', 'All stacks']
    ])
      select.add(new Option(label, value));
    select.value = options.context.sceneStacks || 'off';
    select.onchange = () => options.update({ sceneStacks: select.value });
    controls.append(select);
    const size = document.createElement('input');
    size.type = 'range';
    size.min = 0.15;
    size.max = 1;
    size.step = 0.05;
    size.value = options.context.sceneStackScale || 0.35;
    size.setAttribute('aria-label', 'Scene stack size');
    size.title = 'Scene stack size';
    size.onchange = () => options.update({ sceneStackScale: Number(size.value) });
    controls.append(size);
    let signature = '',
      groups = [];
    const views = new Map();
    function update(projectPoint, zoom = 20) {
      const data = options.data(),
        selected = new Set(options.selected()),
        mode = options.context.sceneStacks || 'off',
        chosen = fixtures(data.project, mode, selected),
        key = JSON.stringify([
          data.snapshot.revision,
          chosen.map((f) => f.id),
          options.catalog().map((b) => b.key),
          options.context.fractal,
          options.context.square
        ]);
      if (key !== signature) {
        signature = key;
        layer.replaceChildren();
        groups = [];
        let budget = 600,
          omitted = 0;
        for (const f of chosen) {
          const layers = data.project.content[f.id] || [],
            local = views.get(f.id) || {
              layer: Math.min(options.context.layer || 0, Math.max(0, layers.length - 1)),
              folded: new Set()
            };
          views.set(f.id, local);
          local.layer = Math.min(local.layer, Math.max(0, layers.length - 1));
          const stack = layers[local.layer],
            nodes = count(stack?.root);
          if (nodes > budget) {
            omitted++;
            continue;
          }
          budget -= nodes;
          const group = document.createElement('section');
          group.className = 'scene-stack';
          group.dataset.sceneStackFixture = f.id;
          group.setAttribute('aria-label', 'Stack for ' + f.name);
          const world = document.createElement('div');
          world.className = 'layer-world';
          const empty = document.createElement('div');
          empty.hidden = true;
          const label = document.createElement('div');
          label.className = 'scene-stack-label';
          const title = document.createElement('button');
          title.textContent = f.name;
          SchemaAuthoring.markFixture(title, f);
          title.onclick = () => options.action('select', f.id, local.layer, stack?.root?.id || 0);
          label.append(title);
          const picker = document.createElement('select');
          picker.setAttribute('aria-label', 'Stack layer for ' + f.name);
          layers.forEach((_, i) => picker.add(new Option('L' + (i + 1), i)));
          picker.value = local.layer;
          picker.onchange = () => {
            local.layer = Number(picker.value);
            local.folded.clear();
            signature = '';
            update(projectPoint, zoom);
          };
          label.append(picker);
          group.append(world, empty, label);
          layer.append(group);
          const draw = () => {
            const root = (options.data().project.content[f.id] || [])[local.layer]?.root;
            const rows = g.SchemaStackLayout.geometry(root, local.folded, options.context);
            options.draw(root, {
              world,
              empty,
              surface: group,
              fixture: f.id,
              layer: local.layer,
              view: {
                pan: { x: 0, y: 0 },
                scale: 1,
                folded: local.folded,
                fractal: options.context.fractal,
                square: options.context.square
              }
            });
            if (!root) {
              const add = document.createElement('button');
              add.textContent = '+ Create stack';
              add.onclick = () => options.action('root', f.id, local.layer, 0);
              world.append(add);
            }
            for (const card of world.querySelectorAll('.block[data-node]')) {
              const id = Number(card.dataset.node),
                act = (kind, extra) => options.action(kind, f.id, local.layer, id, extra);
              card.onpointerdown = (e) => e.stopPropagation();
              card.onclick = (e) => {
                e.stopPropagation();
                if (!e.target.closest('button')) act('select');
              };
              card.tabIndex = 0;
              card.setAttribute('aria-label', f.name + ' · ' + card.title);
              card.onkeydown = (e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  e.stopPropagation();
                  act('select');
                }
              };
              const grip = card.querySelector('.node-grip');
              grip.title = 'Move block or branch…';
              grip.setAttribute('aria-label', 'Move block or branch');
              grip.onclick = (e) => {
                e.stopPropagation();
                act('move');
              };
              card.querySelector('.node-delete').onclick = (e) => {
                e.stopPropagation();
                act('delete');
              };
              const fold = card.querySelector('.fold');
              if (fold) {
                const toggle = (e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  local.folded.has(id) ? local.folded.delete(id) : local.folded.add(id);
                  entry.rows = draw();
                };
                fold.onclick = toggle;
                fold.onkeydown = (e) => {
                  if ((e.key === 'Enter' || e.key === ' ') && !e.repeat) toggle(e);
                };
              }
              for (const port of card.querySelectorAll('.port'))
                port.onclick = (e) => {
                  e.stopPropagation();
                  act(
                    port.classList.contains('below-slot') ? 'below' : 'input',
                    Number(port.dataset.index)
                  );
                };
            }
            return rows;
          };
          const entry = { f, group, world, label, rows: [] };
          entry.rows = draw();
          groups.push(entry);
        }
        if (omitted) {
          const notice = document.createElement('p');
          notice.className = 'scene-stacks-notice';
          notice.textContent =
            omitted +
            ' stacks exceed the600-block view budget. Select fewer fixtures to edit them.';
          layer.append(notice);
        }
      }
      for (const item of groups) {
        const point = projectPoint(item.f.position),
          scale = Math.max(
            0.03,
            Math.min(3, ((options.context.sceneStackScale || 0.35) * zoom) / 20)
          ),
          p = pose(item.rows, point, scale);
        item.world.style.transform = `translate(${p.x}px,${p.y}px) scale(${scale})`;
        item.label.style.transform = `translate(${point.x}px,${point.y + 8}px) translateX(-50%)`;
        const focus=options.focus();
        for(const card of item.world.querySelectorAll('.block[data-node]'))card.classList.toggle('selected',focus.fixture===item.f.id&&focus.layer===views.get(item.f.id).layer&&focus.node===Number(card.dataset.node));
        item.group.classList.toggle('selected', selected.has(item.f.id));
        item.group.style.zIndex = selected.has(item.f.id) ? '3' : '1';
      }
    }
    return { update };
  }
  const api = { fixtures, count, pose, mount };
  g.SchemaSceneStacks = api;
  if (typeof module !== 'undefined') module.exports = api;
})(globalThis);
