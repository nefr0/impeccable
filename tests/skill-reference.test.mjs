import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('..', import.meta.url));

describe('skill reference authoring contracts', () => {
  it('keeps direction contracts in development-only surface briefs', () => {
    const newWork = readFileSync(join(ROOT, 'skill/reference/new-work.md'), 'utf-8').replace(/\r\n?/g, '\n');
    const recordDecision = newWork.match(/## 5\. Record the decision\n([\s\S]*?)\n## 6\./)?.[1] ?? '';

    assert.match(recordDecision, /development-only contract/);
    assert.match(recordDecision, /under `## Direction contract` in the relevant surface brief/);
    assert.match(recordDecision, /read the brief once more/i);
    assert.match(recordDecision, /all six contract blocks/);
    assert.doesNotMatch(recordDecision, /seed/i);

    for (const block of ['THESIS', 'OWN-WORLD', 'STORY', 'FIRST VIEWPORT', 'FORM', 'FINISH']) {
      assert.match(recordDecision, new RegExp(`${block}:`));
    }

    for (const browserArtifact of [
      /HTML or framework comments/,
      /hidden DOM/,
      /<template>/,
      /`data-\*` attributes/,
      /serialized props or state/,
      /React Server Component payloads/,
      /client bundles/,
      /metadata or JSON-LD/,
      /accessibility-only text/,
    ]) {
      assert.match(recordDecision, browserArtifact);
    }

    assert.match(recordDecision, /Never copy the direction contract into implementation source or any browser-delivered artifact/);
    assert.doesNotMatch(newWork, /contract in the artifact's opening comment/);
    assert.doesNotMatch(newWork, /survives the production build/);
    assert.doesNotMatch(newWork, /grep the built output/);
    assert.doesNotMatch(newWork, /emitted markup/);
    assert.doesNotMatch(newWork, /first child of the document's body/);
  });

  it('collects the existing-DESIGN.md choice before the documenter writes', () => {
    const newWork = readFileSync(join(ROOT, 'skill/reference/new-work.md'), 'utf-8').replace(/\r\n?/g, '\n');
    const documenter = readFileSync(join(ROOT, 'skill/agents/impeccable-documenter.md'), 'utf-8').replace(/\r\n?/g, '\n');
    const handoff = newWork.match(/After the last correction[\s\S]*?<!-- rule:skill-documenter-records-the-world -->/)?.[0] ?? '';
    const inputContract = documenter.match(/## Input Contract\n([\s\S]*?)\n## Workflow/)?.[1] ?? '';

    assert.match(handoff, /\{\{ask_instruction\}\} Offer refresh, overwrite, or merge/);
    assert.ok(handoff.indexOf('{{ask_instruction}}') < handoff.indexOf('spawn the shipped documenter'), 'the choice is collected before the documenter is spawned');
    assert.match(inputContract, /refresh, overwrite, or merge/);
    assert.doesNotMatch(documenter, /update, not replace/);
    assert.match(documenter, /no choice arrived, write nothing/);
  });

  it('skips browser evidence and its degraded banner on native critique targets', () => {
    const critique = readFileSync(join(ROOT, 'skill/reference/critique.md'), 'utf-8').replace(/\r\n?/g, '\n');
    const orchestration = critique.match(/### Assessment Orchestration\n([\s\S]*?)\n<codex>/)?.[1] ?? '';

    assert.match(orchestration, /\*\*Native platforms\*\* \(`ios` \/ `android` \/ `adaptive`\): Assessment B's browser evidence does not apply/);
    assert.match(orchestration, /skip both sub-agent gates below, the Codex permission question included/);
    assert.match(critique, /- Native: `Method: design review plus craft-floor source scan \(native platform\)`/);
  });

  it('keeps reduced-motion guidance on the animation build path', () => {
    const animate = readFileSync(join(ROOT, 'skill/reference/animate.md'), 'utf-8').replace(/\r\n?/g, '\n');
    const accessibility = animate.match(/## Accessibility and control\n([\s\S]*?)\n## Verify/)?.[1] ?? '';
    const verify = animate.match(/## Verify\n([\s\S]*?)(?:\n## |$)/)?.[1] ?? '';

    assert.match(accessibility, /prefers-reduced-motion/);
    assert.match(accessibility, /intentional alternative/);
    assert.match(accessibility, /not disabling all motion/);
    assert.match(verify, /reduced[- ]motion/i);
  });

  it('has Setup read PRODUCT.md and DESIGN.md itself, with no engine loader', () => {
    const skill = readFileSync(join(ROOT, 'skill/SKILL.src.md'), 'utf-8').replace(/\r\n?/g, '\n');
    const setup = skill.match(/## Setup\n([\s\S]*?)\n## How to design/)?.[1] ?? '';

    assert.match(setup, /Read the project context yourself; there is no loader/);
    assert.match(setup, /<!-- rule:skill-setup-read-context -->/);
    assert.match(setup, /PRODUCT\.md/);
    assert.match(setup, /DESIGN\.md/);
    assert.doesNotMatch(skill, /impeccable context/);
  });

  it('has new-work write directions itself from the mode file and ask through the harness', () => {
    const newWork = readFileSync(join(ROOT, 'skill/reference/new-work.md'), 'utf-8').replace(/\r\n?/g, '\n');

    for (const mode of ['persuade', 'operate', 'read']) {
      assert.match(newWork, new RegExp(`\\[mode-${mode}\\.md\\]\\(mode-${mode}\\.md\\)`));
    }
    assert.match(newWork, /read the `## Directions` section of the mode's file/);
    assert.match(newWork, /\{\{ask_instruction\}\}/);
    assert.doesNotMatch(newWork, /decision page|serve-question|concept-seed|comp-spec|comp-diff|impeccable context/);
  });

  it('keeps touch-gesture verification in the adapt, audit, and harden references', () => {
    const adapt = readFileSync(join(ROOT, 'skill/reference/adapt.md'), 'utf-8').replace(/\r\n?/g, '\n');
    const audit = readFileSync(join(ROOT, 'skill/reference/audit.md'), 'utf-8').replace(/\r\n?/g, '\n');
    const harden = readFileSync(join(ROOT, 'skill/reference/harden.md'), 'utf-8').replace(/\r\n?/g, '\n');
    const verifyAdaptations = adapt.match(/## Verify Adaptations\n([\s\S]*?)\n## /)?.[1] ?? '';
    const responsive = audit.match(/### 4\. Responsive Design\n([\s\S]*?)\n### 5\./)?.[1] ?? '';
    const edgeCases = harden.match(/### Edge Cases & Boundary Conditions\n([\s\S]*?)\n### /)?.[1] ?? '';
    const verifyHardening = harden.match(/## Verify Hardening\n([\s\S]*?)(?:\n## |$)/)?.[1] ?? '';

    assert.match(verifyAdaptations, /\*\*Primary gesture\*\*/);
    assert.match(verifyAdaptations, /produced the evidence/);
    assert.match(verifyAdaptations, /verify layout, never a gesture/);
    assert.match(verifyAdaptations, /reported gap, not a blocker/);
    assert.match(verifyAdaptations, /\*\*Scroll across it\*\*[\s\S]*without activating it/);
    assert.match(responsive, /\*\*Broken touch interaction\*\*/);
    assert.match(responsive, /what stayed untested/);
    assert.match(responsive, /Exercise the gesture when a browser tool can synthesize touch/);
    assert.match(edgeCases, /\*\*Interrupted gestures\*\*[\s\S]*works without a reload/);
    assert.match(edgeCases, /clear the dragging state and release capture/);
    assert.match(verifyHardening, /\*\*Interrupted gestures\*\*/);
  });

  it('keeps audit score-table dimensions aligned with their numbered sections', () => {
    for (const filename of ['audit.md', 'audit.native.md']) {
      const audit = readFileSync(join(ROOT, `skill/reference/${filename}`), 'utf-8').replace(/\r\n?/g, '\n');
      const sections = [...audit.matchAll(/^### ([1-5])\. (.+?)(?: \([^\n]+\))?$/gm)].map((match) => [
        match[1],
        match[2],
      ]);
      const scores = [...audit.matchAll(/^\| ([1-5]) \| ([^|]+?) \|/gm)].map((match) => [
        match[1],
        match[2].trim(),
      ]);

      assert.deepEqual(scores, sections, filename);
    }
  });
});

describe('mode rule files', () => {
  it('each shipped mode file has exactly the Directions and Comps sections the engine prints', () => {
    for (const mode of ['persuade', 'operate', 'read']) {
      const text = readFileSync(join(ROOT, `skill/reference/mode-${mode}.md`), 'utf-8').replace(/\r\n?/g, '\n');
      const headings = text.split('\n').filter((line) => line.startsWith('## '));
      assert.deepEqual(headings, ['## Directions', '## Comps'], `mode-${mode}.md`);
      for (const section of text.split(/^## (?:Directions|Comps)\n/m).slice(1)) {
        assert.ok(section.trim().length > 200, `mode-${mode}.md has an empty or stub section`);
      }
    }
  });
});
