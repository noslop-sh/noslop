# Competitive Analysis: Beads & Superpowers

## Executive Summary

This analysis examines two prominent AI agent tooling projects—**Beads** (task/memory management) and **Superpowers** (workflow/skills)—and how their concepts relate to noslop's mission of teaching AI agents codebase conventions.

**Key Finding**: Both projects solve different parts of the same problem: agents lack persistent context about *what to do* (Beads) and *how to do it properly* (Superpowers). Noslop addresses a third dimension: *what rules to follow*. Together, these form a complete agent guidance system.

---

## Project Summaries

### Beads (steveyegge/beads)

**What it does**: Distributed, git-backed graph issue tracker designed for AI coding agents. Provides persistent structured memory to manage long-horizon tasks without losing context.

**Core mechanism**: Tasks stored as JSONL in `.beads/` directory, with dependency tracking, collision-free IDs, and semantic "memory decay" for completed tasks.

**Stats**: 8,300+ stars, 506 forks, 117 contributors

### Superpowers (obra/superpowers)

**What it does**: Composable skills framework that guides Claude Code agents through structured phases—brainstorming, planning, implementation, and review.

**Core mechanism**: Slash commands that invoke skills (markdown instruction sets) at different workflow stages. Enforces test-driven development and systematic processes.

**Stats**: 14,000+ stars, 1,100+ forks, 12 contributors

### noslop (our project)

**What it does**: Pre-commit feedback loop that teaches agents codebase-specific conventions via checks and verifications.

**Core mechanism**: File pattern matching triggers contextual guidance at commit time. Agents must verify they addressed each check before the commit proceeds.

---

## What People Like

### Beads - Strengths

| Strength | Evidence |
|----------|----------|
| **Solves a real pain point** | "The adoption is fast because the pain is real. Anyone who's spent hours re-explaining context to their coding agent every morning gets it immediately." |
| **Git-native storage** | Version control for tasks, branch-per-feature workflows, no separate database |
| **Agent-optimized output** | JSON format, `bd ready` command shows unblocked work—designed for programmatic consumption |
| **Lightweight** | Single Go binary, fast local SQLite cache |
| **Dependency graph** | Agents can understand task relationships and tackle unblocked work first |
| **Memory decay** | Automatic summarization of closed tasks preserves context window |

### Superpowers - Strengths

| Strength | Evidence |
|----------|----------|
| **Forces structured thinking** | "The agent doesn't just jump into trying to write code. Instead, it steps back and asks you what you're really trying to do." |
| **Blocks skip-ahead behavior** | "These skills literally block you from skipping steps. No more 'I think it works' without proof." |
| **Token efficient** | "The core is VERY token light - it pulls in one doc of fewer than 2k tokens" |
| **Composable skills** | Individual skills can be invoked independently or combined |
| **Enforces TDD** | RED-GREEN-REFACTOR cycles built into the workflow |
| **Two-stage code review** | Systematic quality checks before completion |

### noslop - Strengths (for reference)

| Strength | Mechanism |
|----------|-----------|
| **Captures institutional knowledge** | `.noslop.toml` grows into living documentation |
| **Commit-time interception** | Catches issues before code review round-trip |
| **Agent self-correction** | Agents fix issues themselves after seeing guidance |
| **Audit trail** | Verifications recorded as git trailers |
| **Already has task system** | Built-in dependency-aware task tracking |

---

## What People Dislike / Pain Points

### Beads - Weaknesses

| Weakness | Evidence |
|----------|----------|
| **AI-generated documentation** | Issue #376: "I want to love Beads but the AI generated docs make it impossible" |
| **CLI complexity** | Issue #780: "overwhelming number of commands and flags" |
| **Sync data loss risk** | Issue #911: "bd sync export-before-pull causes data loss" |
| **Onboarding friction** | Multiple issues about setup difficulties (#903, #298, #275) |
| **"Crummy architecture"** | Yegge himself admits it "requires AI to work around edge cases where it breaks" |
| **Merge conflict potential** | Despite collision-free IDs, multi-agent sync remains problematic |

### Superpowers - Weaknesses

| Weakness | Evidence |
|----------|----------|
| **Windows compatibility** | Issues #232, #225: Path resolution failures, hook problems on Windows |
| **Token limit overflows** | Issue #227: "Write-plan exceeding token limits" |
| **Skill quality variance** | "using-superpowers" skill scored 68/100 (Grade D) |
| **Limited AI provider support** | Issue #217: No GitHub Copilot support, Claude-specific |
| **Rigid workflows** | Some users find the forced structure too constraining |
| **Pedantic spec reviewer** | Issue #221: "Spec Reviewer Pedantry" suggests over-strictness |

---

## Relationship to noslop

### Conceptual Overlap

```
┌─────────────────────────────────────────────────────────────────┐
│                    AI Agent Guidance Stack                       │
├─────────────────────────────────────────────────────────────────┤
│  WHAT TO DO        │  HOW TO DO IT      │  WHAT RULES TO FOLLOW │
│  (Beads)           │  (Superpowers)     │  (noslop)             │
├────────────────────┼────────────────────┼───────────────────────┤
│  Task tracking     │  Workflow stages   │  Check patterns       │
│  Dependencies      │  Skills/processes  │  Severity levels      │
│  Priority/ready    │  TDD enforcement   │  File targeting       │
│  Memory/context    │  Code review       │  Verification trail   │
└────────────────────┴────────────────────┴───────────────────────┘
```

### Natural Integration Points

1. **Beads → noslop**: When a Beads task involves files matching noslop checks, the agent gets convention guidance at commit time
2. **Superpowers → noslop**: The "execute-plan" phase could integrate noslop checks to ensure implementations follow conventions
3. **noslop → Beads**: Blocking checks could auto-create follow-up Beads tasks when agents need to address issues
4. **Task overlap**: noslop already has a task system (`noslop task`) with dependencies—potential to consolidate or bridge

### Differentiation

| Aspect | Beads | Superpowers | noslop |
|--------|-------|-------------|--------|
| **Trigger** | Explicit task ops | Slash commands | Git commit |
| **Storage** | `.beads/` JSONL | Skills markdown | `.noslop.toml` |
| **Focus** | What to work on | How to work | What to check |
| **Enforcement** | Advisory | Process gates | Commit blocking |
| **Scope** | Cross-session | Single session | Per-commit |

---

## Synthesis Opportunities

### Concepts to Absorb (Low Bloat, High Value)

#### From Beads

1. **`ready` concept for checks**
   - Show which pending verifications can be addressed now vs. are blocked by others
   - Minimal addition: one flag to `noslop check list --ready`

2. **JSON output mode** (already implemented!)
   - noslop already has `--json` flag
   - Ensures agent-friendly programmatic consumption

3. **Memory decay / summarization**
   - When verification history grows large, auto-summarize old entries
   - Keeps context window manageable for agents

4. **Collision-free IDs**
   - Current IDs are sequential (DB-1, DB-2)
   - Hash-based IDs would prevent conflicts in multi-agent scenarios

#### From Superpowers

1. **Pre-commit "checklist" skill concept**
   - A `.noslop/skills/` directory with markdown guidance documents
   - Checks could reference skills: `skill = "migrations.md"`
   - Richer context than single-line messages

2. **Verification commands**
   - Checks could include verification: `verify = "alembic heads"`
   - Agent can run command to confirm compliance before verifying

3. **Staged verification workflow**
   - Like Superpowers' plan → execute → review
   - `noslop check run` → `noslop verify` → `git commit`

### Concepts to Avoid (High Bloat, Low Value for noslop)

| Concept | Why Avoid |
|---------|-----------|
| Full task management (Beads-style) | noslop already has basic tasks; full graph tracker is scope creep |
| Workflow orchestration (Superpowers) | noslop is a single-purpose tool (commit-time); workflows belong elsewhere |
| Memory/context persistence | noslop's context is `.toml` file; adding database/cache adds complexity |
| Skills/slash commands | These are IDE/agent features, not pre-commit tooling |
| AI provider integrations | noslop is agent-agnostic; let integrations happen externally |
| Sync infrastructure | Git already handles this; custom sync adds Beads' biggest pain points |

---

## Recommended Strategy

### Phase 1: Strengthen Core (No New Features)

- Improve documentation (avoid Beads' AI-doc trap)
- Ensure Windows compatibility (learn from Superpowers issues)
- Keep CLI simple (avoid Beads' command explosion)

### Phase 2: Minimal Strategic Additions

1. **Verification commands for checks**
   ```toml
   [[check]]
   target = "migrations/versions/*.py"
   message = "Generated with alembic revision --autogenerate?"
   verify = "alembic heads"
   severity = "block"
   ```

2. **Skill references for richer context**
   ```toml
   [[check]]
   target = "api/public/*_router.py"
   message = "Rate limiting decorator added?"
   skill = "rate-limiting.md"  # .noslop/skills/rate-limiting.md
   severity = "block"
   ```

3. **Hash-based check IDs**
   - Prevent merge conflicts in team scenarios
   - `ns-a3f8` instead of `DB-1`

### Phase 3: Integration, Not Absorption

Rather than absorbing Beads/Superpowers features:

- **Publish MCP server** for noslop (like Beads' beads-mcp)
- **Document integration patterns** with Beads/Superpowers
- **Stay focused** on the commit-time feedback loop

---

## Conclusion

Beads and Superpowers are complementary to noslop, not competitive. Each addresses a different phase of agent-assisted development:

1. **Beads**: Planning what to do (persistent memory)
2. **Superpowers**: Guiding how to do it (workflow skills)
3. **noslop**: Enforcing what rules to follow (commit-time conventions)

The strongest path forward is **focused excellence** in the commit-time feedback loop, with **lightweight bridges** to the broader ecosystem rather than feature absorption. Users who want full task management will use Beads. Users who want workflow skills will use Superpowers. Both will benefit from noslop catching convention violations at commit time.

**The mission remains**: Teach AI agents your codebase's unwritten rules—at the moment it matters most.

---

## Sources

- [Beads GitHub Repository](https://github.com/steveyegge/beads)
- [Superpowers GitHub Repository](https://github.com/obra/superpowers)
- [Beads Best Practices - Steve Yegge](https://steve-yegge.medium.com/beads-best-practices-2db636b9760c)
- [Introducing Beads - Steve Yegge](https://steve-yegge.medium.com/introducing-beads-a-coding-agent-memory-system-637d7d92514a)
- [Superpowers: How I'm using coding agents - Jesse Vincent](https://blog.fsck.com/2025/10/09/superpowers/)
- [Claude Code Skills - Medium](https://medium.com/@ooi_yee_fei/claude-code-skills-superpowering-claude-code-agents-a42b44a58ae2)
- [How I force Claude Code to plan - Trevor Lasn](https://www.trevorlasn.com/blog/superpowers-claude-code-skills)
- [Beads Guide - Better Stack](https://betterstack.com/community/guides/ai/beads-issue-tracker-ai-agents/)
