import fs from 'fs';
import { optimize } from 'svgo';
import outlineStroke from 'svg-outline-stroke';

const SRC = '/Users/fredrikstahre/sites/markdown-editor/assets/icons/editor icons';
fs.mkdirSync('work/raw', { recursive: true });

// Target stem as a fraction of the icon's larger dimension. The width-2 icons
// in this set already sit at ~5.5% once their group scale is accounted for;
// this normalizes the width-3 outliers (italic, code-block) down to match.
const STEM = 0.055;

const ICONS = ['bold','italic','code-block','quote','link','unlink',
  'list-bullet-unordered','list-bullet-numbers','list-bullet-letters',
  'outline','anfang','erase','save','print','copy','cut','paste'];

// svgo leaves the Illustrator group transforms in place, so a declared
// stroke-width is multiplied by the group scale before it renders. Recover
// that factor so stroke widths can be set in real user units.
function groupScale(svg) {
  const vals = [
    ...[...svg.matchAll(/matrix\(\s*(-?[\d.]+)[,\s]/g)].map(m => Math.abs(+m[1])),
    ...[...svg.matchAll(/scale\(\s*(-?[\d.]+)/g)].map(m => Math.abs(+m[1])),
  ].filter(v => v > 1.001);
  return vals.length ? Math.max(...vals) : 1;
}

for (const name of ICONS) {
  let svg = fs.readFileSync(`${SRC}/${name}.svg`, 'utf8');
  const vb = svg.match(/viewBox="([\d.\s-]+)"/)[1].trim().split(/\s+/).map(Number);
  const maxDim = Math.max(vb[2], vb[3]);
  const gs = groupScale(svg);
  svg = svg.replace(/width="100%"/, `width="${vb[2]}"`).replace(/height="100%"/, `height="${vb[3]}"`);

  const declared = (STEM * maxDim / gs).toFixed(4);
  svg = svg.replace(/stroke-width:[\d.]+/g, `stroke-width:${declared}`)
           .replace(/stroke-width="[\d.]+"/g, `stroke-width="${declared}"`);

  const r = optimize(svg, { multipass: true, plugins: [{ name: 'preset-default' }] });
  // Purely filled icons have no strokes to expand; running them through the
  // outliner only introduces stray slivers, so pass them straight through.
  const hasStroke = /stroke-width/.test(r.data);
  const out = hasStroke
    ? await outlineStroke(r.data, { optCurve: false, steps: 4, round: 3, fixIndividual: true })
    : r.data;
  const ds = [...out.matchAll(/\sd="([^"]+)"/g)].map(m => m[1]);
  if (!ds.length) { console.error(`FAIL ${name}`); continue; }
  fs.writeFileSync(`work/raw/${name}.path`, ds.join(' '));
  console.log(`${name.padEnd(24)} vb=${maxDim} groupScale=${gs} declared=${declared} effective=${(declared*gs).toFixed(1)} (${((declared*gs/maxDim)*100).toFixed(1)}%)`);
}
