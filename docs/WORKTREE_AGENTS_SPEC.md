# Multi-Agent Implementation Specification

## Overview

This document specifies the implementation of multi-agent support for noslop using git worktrees. The core idea is:

**One Agent = One Git Worktree**

Each AI agent (Claude, Codex, etc.) works in its own git worktree, with its own branch, isolated from other agents. Agents are reusable across tasks - they can be assigned to different tasks over time. Tasks are claimed to prevent double-assignment.

## Key Design Decisions

1. **Agent terminology**: Users interact with "agents" not "worktrees" - the git worktree is an implementation detail
2. **Agents are reusable**: Agents can be reassigned to different tasks - they're not tied to a specific task
3. **Task claiming**: Tasks have a `claimed_by` field to prevent multiple agents from working on the same task
4. **Per-agent state**: Each agent has its own HEAD, staged verifications, and current topic
5. **Shared task pool**: All agents see the same tasks from `.noslop/refs/tasks/`
6. **tmux integration**: Agent processes are managed via tmux sessions

## Storage Layout

```
repo/                                    # Main worktree
├── .noslop.toml                        # SHARED: Committed config
│                                       #   - checks, topics, prefix
│                                       #   - [agent] config section
│
└── .noslop/                            # Local state (gitignored)
    │
    ├── refs/tasks/                     # SHARED: Task definitions (JSON files)
    │   ├── TSK-1                       # Each task includes claimed_by field
    │   ├── TSK-2
    │   └── TSK-3
    │
    └── agents/                         # Agent workspaces (git worktrees)
        │
        ├── agent-1/                    # Agent 1's isolated workspace
        │   ├── .noslop/
        │   │   ├── HEAD               # Contains current task ID
        │   │   └── staged-verifications.json
        │   ├── src/
        │   └── ... (full repo copy)
        │
        └── agent-2/                    # Agent 2's isolated workspace
            ├── .noslop/
            │   └── HEAD               # Contains current task ID
            └── ...
```

### Shared vs Local State

| Path | Scope | Contents |
|------|-------|----------|
| `.noslop.toml` | Shared (committed) | Checks, topics, prefix, agent config |
| `.noslop/refs/tasks/` | Shared (main worktree) | Task definitions, visible to all agents |
| `.noslop/agents/` | Shared (main worktree) | Contains all agent worktrees |
| `.noslop/HEAD` | **Per-agent** | Current task for THIS agent only |
| `.noslop/staged-verifications.json` | **Per-agent** | Pending verifications for THIS agent |
| `.noslop/current-topic` | **Per-agent** | Selected topic for THIS agent |

---

## Implementation Summary

### Files Modified

#### `src/paths.rs`

Added agent path functions:
- `agent_root()` - Current agent's worktree directory
- `agent_local_dir()` - Per-agent `.noslop/` directory
- `agents_dir()` - Directory containing all agents (`.noslop/agents/`)
- `agent_path(name)` - Path to a specific agent's workspace
- `is_agent()` - Check if running inside an agent workspace

Changed to use per-agent paths:
- `head_file()` - Now returns agent-local HEAD
- `staged_verifications()` - Now per-agent
- `current_topic_file()` - Now per-agent

#### `src/git/mod.rs`

Added worktree management functions:
- `get_current_worktree()` - Get current worktree root
- `is_linked_worktree()` - Check if in a linked worktree
- `list_worktrees()` - List all worktrees
- `create_worktree(path, branch)` - Create new worktree
- `remove_worktree(path, force)` - Remove worktree
- `get_branch_in(path)` - Get branch name in a worktree

#### `src/storage/refs.rs`

Added to `TaskRef`:
- `claimed_by: Option<String>` - Agent name that claimed this task

Added methods to `TaskRefs`:
- `claim(id, agent_name)` - Claim task for an agent
- `release(id)` - Release claim on a task
- `is_claimed(id)` - Check if task is claimed
- `get_claimed_by(id)` - Get claiming agent name
- `list_by_agent(agent_name)` - List tasks by agent
- `list_unclaimed()` - List available tasks

Modified:
- `next_pending_unblocked()` - Now skips claimed tasks
- `complete()` - Releases claim when completing

#### `src/noslop_file.rs`

Added `AgentConfig` struct:
```rust
pub struct AgentConfig {
    pub command: String,      // Default: "claude"
    pub use_tmux: bool,       // Default: true
    pub args: Vec<String>,    // Additional args
    pub auto_assign: bool,    // Auto-assign tasks on spawn
}
```

#### `src/commands/agent.rs` (New)

Implements agent subcommands:
- `spawn` - Create new agent with optional task assignment
- `list` - List all agents
- `kill` - Stop agent and remove workspace
- `status` - Show current agent status
- `assign` - Assign task to existing agent

#### `src/cli.rs`

Added `AgentAction` enum with:
- `Spawn { name, count, no_tmux, idle }`
- `List`
- `Kill { name, force }`
- `Status`
- `Assign { name, task_id }`

---

## CLI Reference

```bash
# Spawn agents
noslop agent spawn                    # Spawn with auto-name, auto-assign task
noslop agent spawn --name my-agent    # Spawn with specific name
noslop agent spawn --count 5          # Spawn 5 agents at once
noslop agent spawn --idle             # Spawn without assigning a task
noslop agent spawn --no-tmux          # Don't start tmux session

# List agents
noslop agent list                     # Show all agents
noslop --json agent list              # JSON output

# Manage agents
noslop agent status                   # Show current agent status
noslop agent assign agent-1 TSK-3     # Assign task to agent
noslop agent kill agent-1             # Stop agent
noslop agent kill agent-1 --force     # Force remove with uncommitted changes
```

---

## Example Workflow

```bash
# 1. Create some tasks
$ noslop task add "Implement OAuth login" -p p0
Created: TSK-1
$ noslop task add "Fix database connection pooling" -p p1
Created: TSK-2
$ noslop task add "Add rate limiting middleware" -p p2
Created: TSK-3

# 2. Spawn multiple agents (auto-assigns pending tasks)
$ noslop agent spawn --count 3
Spawned agent: agent-1
  Path:   .noslop/agents/agent-1
  Branch: agent/agent-1/tsk-1
  Task:   TSK-1 - Implement OAuth login
  Tmux:   noslop-agent-1

Spawned agent: agent-2
  Path:   .noslop/agents/agent-2
  Branch: agent/agent-2/tsk-2
  Task:   TSK-2 - Fix database connection pooling
  Tmux:   noslop-agent-2

Spawned agent: agent-3
  Path:   .noslop/agents/agent-3
  Branch: agent/agent-3/tsk-3
  Task:   TSK-3 - Add rate limiting middleware
  Tmux:   noslop-agent-3

To attach to an agent:
  tmux attach -t noslop-<name>

# 3. See what's running
$ noslop agent list
AGENT           TASK            STATUS       BRANCH                         TMUX
-------------------------------------------------------------------------------------
agent-1         TSK-1           in_progress  agent/agent-1/tsk-1            noslop-agent-1
agent-2         TSK-2           in_progress  agent/agent-2/tsk-2            noslop-agent-2
agent-3         TSK-3           in_progress  agent/agent-3/tsk-3            noslop-agent-3

# 4. Attach to watch an agent work
$ tmux attach -t noslop-agent-1

# 5. When an agent finishes, assign a new task
$ noslop agent assign agent-1 TSK-4
Assigned task TSK-4 to agent agent-1
  Title: Write documentation

# 6. Clean up agents when done
$ noslop agent kill agent-1
Agent killed: agent-1
  Released task: TSK-4
  Removed workspace: .noslop/agents/agent-1
```

---

## Configuration

Add to `.noslop.toml`:

```toml
[agent]
command = "claude"        # AI agent command to run
use_tmux = true           # Use tmux for session management
auto_assign = true        # Auto-assign tasks when spawning
args = []                 # Additional args for agent command
```
