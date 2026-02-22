# ADR-0000: MADR Template for ADR

## Status

Accepted

---

## Template

```markdown
# ADR-XXXX: Title

## Status

Accepted | Proposed | Deprecated | Superseded by ADR-XXXX

## Context and Problem Statement

What problem are we solving and why does it need a decision?

## Decision Drivers

- Bullet list of forces, constraints, and goals that shaped the decision

## Considered Options

1. Option A
2. Option B
3. Option C

## Decision Outcome

Chosen option N: **Name**, because [brief justification].

## [Additional sections as needed]

Rationale, folder structure, sequence flows, etc. Use headings to organise.

## Diagram

\`\`\`mermaid
// Only include if it genuinely clarifies a flow or structure.
// Good candidates: command lifecycle sequence, state machine, folder hierarchy,
// relay trigger chain. Skip for simple decisions.
\`\`\`

## Consequences

### Positive

- ...

### Negative

- ...

### Risks

- ...
```

---

## Rules

1. **File naming.** Use `NNNN-kebab-case-title.md` in `docs/adr/`. Increment the number from the last ADR.

2. **Status values.** Use one of: `Accepted`, `Proposed`, `Deprecated`, `Superseded by ADR-XXXX`.

3. **Context and Problem Statement.** Describe the situation and why a decision is needed. Do not mix in the decision itself.

4. **Considered Options.** List at least two. If only one option was realistic, explain why alternatives were ruled out quickly.

5. **Decision Outcome.** State the chosen option and the key reason. Keep it to one sentence if possible.

6. **Consequences.** Always include Positive, Negative, and Risks subsections, even if brief.

7. **Code examples.** Use Rust (`.rs`) when illustrating structure or patterns.

8. **Mermaid diagrams.** Add a `## Diagram` section only when it genuinely clarifies a flow or structure — command lifecycle sequences, state machines, relay trigger chains, folder hierarchies. Skip for simple decisions. Verify syntax is valid (no missing `end`, mismatched arrows, or unclosed blocks) before finalising.

9. **Architecture patterns.** Where relevant, name the patterns the decision embodies: FCIS, vertical slices, ports and adapters, event sourcing, CQRS, outbox, saga.

10. **Update the index.** Add the new ADR to the table in `docs/adr/architecture-summary.md`.

---

## Verification checklist

Before finalising an ADR:

- [ ] Status is set
- [ ] At least two options are listed
- [ ] Decision Outcome names the chosen option and gives a reason
- [ ] Consequences covers positive, negative, and risks
- [ ] Mermaid diagram syntax is valid, or section is omitted
- [ ] Entry added to the ADR index in `architecture-summary.md`
- [ ] Any cross-references to other ADRs use the `ADR-XXXX` format
