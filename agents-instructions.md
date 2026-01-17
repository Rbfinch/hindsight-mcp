# Agent Workflow Instructions for VS Code

This document describes how to use the agentic workflow system in VS Code for the hindsight-mcp project.

## Overview

The workspace includes a multi-agent system designed to handle complex development tasks. Each agent has a specific role and can hand off work to other agents as needed. The agents work together in a coordinated workflow, typically orchestrated by the Orchestrator agent.

## Available Agents

| Agent | Role | Description |
|-------|------|-------------|
| **Orchestrator** | The Conductor | Decomposes complex objectives, manages state, coordinates workflow |
| **Context** | The Retriever | Explores codebase, gathers relevant context for implementation |
| **Implementation** | The Coder | Synthesizes code based on specifications and context |
| **Verification** | The Tester/Reviewer | Reviews code for quality, security; generates tests |
| **Runtime** | The Executor | Runs compilers, linters, tests; provides feedback |
| **Commit** | The Archivist | Summarizes work, creates detailed commits, pushes to dev |

## Agent Workflow Diagram

```
                    ┌─────────────────┐
                    │   Orchestrator  │
                    │  (The Conductor)│
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│    Context    │──▶│Implementation │──▶│  Verification │
│(The Retriever)│   │  (The Coder)  │   │(The Reviewer) │
└───────────────┘   └───────┬───────┘   └───────┬───────┘
                            │                   │
                            ▼                   ▼
                    ┌───────────────┐   ┌───────────────┐
                    │    Runtime    │──▶│    Commit     │
                    │(The Executor) │   │(The Archivist)│
                    └───────────────┘   └───────────────┘
```

## Using Agents in VS Code

### Starting an Agent

1. Open the GitHub Copilot Chat panel in VS Code (`Cmd+Shift+I` on macOS)
2. Type `@` followed by the agent name to invoke it directly, or
3. Use the Orchestrator for complex tasks that need decomposition

### Invoking Specific Agents

```
@orchestrator <your complex task>
@context <what context do you need?>
@implementation <what to implement>
@verification <what to review>
@runtime <what to execute>
@commit <summarize and commit changes>
```

### Typical Workflow

For a feature implementation:

1. **Start with Orchestrator**: Describe your objective
   ```
   @orchestrator Implement a new parser for git blame output in hindsight-git
   ```

2. **Context Gathering**: Orchestrator hands off to Context agent
   - Context agent explores the codebase
   - Identifies relevant patterns and files
   - Returns structured context

3. **Implementation**: With context, Implementation agent writes code
   - Follows project conventions
   - Produces clean, documented code

4. **Verification**: Implementation is reviewed
   - Code quality checks
   - Security review
   - Test generation

5. **Runtime**: Tests are executed
   - Build verification
   - Test execution
   - Linting checks

6. **Commit**: Final step
   - Comprehensive summary generated
   - Detailed commit message with ISO 8601 timestamp
   - Changes pushed to `dev` branch

## Agent Details

### Orchestrator (The Conductor)

**Purpose**: Central coordinator for complex tasks

**Use when**:
- Task requires multiple steps or agents
- You need a plan before implementation
- Work needs to be decomposed

**Example**:
```
@orchestrator Add support for parsing git stash entries, including tests and documentation
```

### Context (The Retriever)

**Purpose**: Gather codebase knowledge before implementation

**Use when**:
- Starting work in an unfamiliar area
- Need to understand existing patterns
- Looking for related implementations

**Example**:
```
@context What patterns are used for parsing in hindsight-git?
```

### Implementation (The Coder)

**Purpose**: Write code following specifications

**Use when**:
- You have clear requirements
- Context has been gathered
- Ready to write code

**Example**:
```
@implementation Add a new StashEntry struct with parser following the existing Commit pattern
```

### Verification (The Tester/Reviewer)

**Purpose**: Quality gate for code changes

**Use when**:
- Code needs review
- Tests need to be written
- Security review required

**Example**:
```
@verification Review the new stash parsing implementation for correctness and security
```

### Runtime (The Executor)

**Purpose**: Execute build, test, and lint commands

**Use when**:
- Need to run tests
- Want to verify build succeeds
- Running linters/formatters

**Example**:
```
@runtime Run the test suite for hindsight-git
```

### Commit (The Archivist)

**Purpose**: Finalize work with comprehensive commit

**Use when**:
- Implementation is complete and verified
- Ready to commit to `dev` branch
- Need a detailed summary of changes

**Example**:
```
@commit Summarize the stash parsing feature and commit to dev
```

## Best Practices

### Do

- ✅ Start complex tasks with the Orchestrator
- ✅ Let Context agent gather information before implementation
- ✅ Always verify code through the Verification agent
- ✅ Run tests via Runtime agent before committing
- ✅ Use Commit agent as the final step

### Don't

- ❌ Skip context gathering for unfamiliar code areas
- ❌ Commit without verification and testing
- ❌ Use Implementation agent without clear specifications
- ❌ Force push or bypass the dev branch workflow

## Handoff Reference

Each agent can hand off to specific other agents:

| Agent | Can Hand Off To |
|-------|-----------------|
| Orchestrator | Context, Implementation, Verification, Commit |
| Context | Orchestrator, Implementation |
| Implementation | Verification, Runtime, Orchestrator |
| Verification | Implementation, Runtime, Orchestrator, Commit |
| Runtime | Implementation, Orchestrator, Verification, Commit |
| Commit | Orchestrator, Implementation |

## Troubleshooting

### Agent not responding as expected

1. Be more specific in your request
2. Provide additional context
3. Try starting with the Orchestrator for complex tasks

### Verification failing

1. Check the specific issues raised
2. Hand off to Implementation to fix
3. Re-run Verification after fixes

### Commit failing

1. Ensure all tests pass (use Runtime agent)
2. Check for uncommitted conflicts
3. Verify you're on the correct branch

## Files Location

Agent definitions are located in:
```
.github/agents/
├── orchestrator.agent.md
├── context.agent.md
├── implementation.agent.md
├── verification.agent.md
├── runtime.agent.md
└── commit.agent.md
```
