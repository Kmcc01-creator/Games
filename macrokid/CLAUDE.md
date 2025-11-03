# Claude Code Configuration for DevMe Project

This file configures Claude Code behavior and constraints for the DevMe project.

## Documentation Generation Constraints

Based on the documentation audit (2025-10-31), we've identified that ~25% of documentation is redundant boilerplate. Apply these constraints to reduce verbosity while maintaining clarity:

### 0. DO NOT Generate After-Task Summaries

**CRITICAL CONSTRAINT**: Claude Code should NOT automatically generate documentation after completing tasks unless explicitly requested by the user.

**PROHIBITED**:
- Task completion summaries
- Session summaries
- "What we did today" documents
- Progress reports after completing work
- Review documents after analysis
- Multiple overlapping documents about the same topic (e.g., VULKAN_ROBUSTNESS_SUMMARY.md, VULKAN_ROBUSTNESS_REVIEW.md, VULKAN_ROBUSTNESS_VISUAL_GUIDE.md all covering the same content)

**RATIONALE**:
- Documentation should describe **systems and capabilities**, not **activities**
- Task completion is tracked in git commits and pull requests
- Session work is visible in chat history
- Generating summaries after every task creates documentation sprawl
- Users will explicitly request documentation when needed

**WHEN TO DOCUMENT** (only these scenarios):
1. User explicitly requests documentation: "Write a guide for X"
2. New feature/system created that needs explanation for future users
3. Architecture decision that needs recording (ADR)
4. API/interface that needs reference documentation
5. Updating existing documentation to reflect code changes

**WHEN NOT TO DOCUMENT**:
- After completing a bug fix (unless it reveals architectural insight worth documenting)
- After analyzing code (analysis findings can be communicated in chat, not as files)
- After reviewing a system (communicate findings in chat, update existing docs if needed)
- After reorganizing files (no need for "REORGANIZATION_SUMMARY.md")
- After each development session (no "SESSION_SUMMARY.md" files)

**EXAMPLES**:

❌ **BAD** (extraneous documentation):
```
User: "Review the Vulkan renderer for robustness issues"
Assistant: [reviews code, finds issues]
Assistant: [generates VULKAN_ROBUSTNESS_REVIEW.md]
Assistant: [generates VULKAN_ROBUSTNESS_SUMMARY.md]
Assistant: [generates VULKAN_ROBUSTNESS_QUICK_FIX.md]
Assistant: [generates VULKAN_ROBUSTNESS_INDEX.md]
Assistant: [generates VULKAN_ROBUSTNESS_VISUAL_GUIDE.md]
→ VIOLATION: 5 documents for one review task, all overlapping
```

✅ **GOOD** (concise communication):
```
User: "Review the Vulkan renderer for robustness issues"
Assistant: [reviews code, finds 8 issues]
Assistant: [responds in chat with findings and recommendations]
User: "Can you document these findings?"
Assistant: [generates single VULKAN_ROBUSTNESS_REVIEW.md with all info]
```

❌ **BAD** (task summary):
```
User: "Fix the texture loading bug"
Assistant: [fixes bug]
Assistant: [generates TEXTURE_LOADING_FIX_SUMMARY.md]
→ VIOLATION: Documenting the task, not the system
```

✅ **GOOD**:
```
User: "Fix the texture loading bug"
Assistant: [fixes bug]
Assistant: "Fixed texture loading bug in vk_renderer.cpp:234. The issue was..."
[communicates in chat, commits with clear message, no doc file]
```

**KEY PRINCIPLE**: Communicate findings in chat first. Only generate documentation files when:
1. User explicitly requests it
2. The information needs to persist for future developers
3. No existing document covers this information

### 1. No Repetitive Boilerplate

**CONSTRAINT**: Do not generate:
- Multiple "executive summary" sections with identical content repeated across documents
- Table of contents that merely duplicate section headings with no added context
- Duplicate "metadata status" sections in multiple files
- Redundant "related documentation" lists that appear in nearly every file

**EXAMPLE OF VIOLATION**:
```
Document A: "Executive Summary - This project has 167 files..."
Document B: "Executive Summary - This project has 167 files..." (same text)
→ VIOLATION: Generate once, reference with link
```

**CORRECT APPROACH**:
```
Document A: "Executive Summary - see DOCUMENTATION_SYSTEM_SUMMARY.md for overview"
Document B: References Document A instead of repeating
```

### 2. Hierarchical Documentation with Clear Ownership

**CONSTRAINT**: One primary document per topic, with cross-references

**Structure**:
```
TIER 1: Primary document (authoritative, comprehensive)
TIER 2: Quick reference (summary)
TIER 3: Deep dives (specialized topics, referenced from Tier 1)

DO NOT create:
- 5 documents about "configuration" with overlapping content
- Multiple "overview" files covering the same system
- Parallel guides for the same topic
```

**EXAMPLES**:
- ❌ CONFIGURATION_SYSTEM_DESIGN.md + CONFIGURATION_ANALYSIS.md + CONFIGURATION_INDEX.md
- ✅ CONFIGURATION_SYSTEM_DESIGN.md (primary) + CONFIGURATION_QUICK_REFERENCE.md (summary)

### 3. Maximum Document Size Limits

**CONSTRAINT**: Keep documents focused and actionable

```
Type                      Max Size    Rationale
─────────────────────────────────────────────────────
Guide/Tutorial            20 KB       Focused on one task
Architecture              40 KB       Can be comprehensive
Reference/API             30 KB       Lookups, not prose
Analysis                  35 KB       In-depth but focused
Quick Reference            8 KB       Must be brief
Status/Summary            12 KB       Facts, not narrative
```

**GUIDELINE**: If you're exceeding limits:
1. Split into smaller, focused documents
2. Use cross-references instead of duplicating content
3. Move verbose sections to appendices (in separate file)
4. Trim examples (keep 1-2, reference others)

### 4. No Redundant Example Patterns

**CONSTRAINT**: Explain pattern once, reference thereafter

**EXAMPLE**:
- ❌ Document A: Shows YAML metadata format (5 lines)
- ❌ Document B: Shows YAML metadata format again (5 lines)
- ❌ Document C: Shows YAML metadata format again (5 lines)
- ✅ DOCUMENTATION_MANAGEMENT_SYSTEM.md: Define format once
- ✅ Other docs: "See DOCUMENTATION_MANAGEMENT_SYSTEM.md for metadata format"

### 5. Concise Section Formatting

**CONSTRAINT**: Use structured formats to reduce prose

**INSTEAD OF**:
```markdown
The system has three main components. The first component is called
IntentExecutor and it is responsible for executing intents. It has
several methods like execute() and execute_nocache(). The second
component is ArtifactStore...
```

**USE**:
```markdown
## Core Components

| Component | Purpose | Key Methods |
|-----------|---------|-------------|
| IntentExecutor | Execute intents | execute(), execute_nocache() |
| ArtifactStore | Store artifacts | has_artifacts(), get_artifacts() |
```

### 6. Link Instead of Duplicate

**CONSTRAINT**: Cross-reference instead of copying content

**When you have**:
- Documentation that appears in multiple places
- Metadata that's repeated across files
- Status information in multiple docs
- Configuration details spread across files

**DO**: Create one authoritative source and reference it
```markdown
See [Metadata Standard](DOCUMENTATION_MANAGEMENT_SYSTEM.md#metadata-standard) for format details.
```

### 7. Focus on Unique Insights

**CONSTRAINT**: Each document should add new value

**QUESTION TO ASK BEFORE WRITING A SECTION**:
- "Does this section provide information unique to this document?"
- "Is this a summary of another document?"
- "Could this be better served as a link to the original?"

**If answering "no" to first Q and "yes" to last two**: Don't include it.

### 8. Dynamic Content is Fragile

**CONSTRAINT**: Don't manually list things that can change

**EXAMPLES**:
- ❌ "This project has X files organized in Y directories" (will be outdated)
- ✅ "See DOCUMENTATION_CATALOG.md for current file listing"

- ❌ "Phase 1 involves these 4 tasks..." (might change)
- ✅ "See DOCUMENTATION_SYSTEM_ROADMAP.md for phase details"

### 9. Archive Aggressively

**CONSTRAINT**: Move content to ARCHIVE/ after superseding

When a newer version exists:
- Move old version to `/ARCHIVE/`
- Add note at top of archive file: "SUPERSEDED by [new doc]. See for current version."
- Don't delete (might be historically useful)
- Don't keep in active directory (clutters navigation)

**EXAMPLES**:
```
ACTIVE: CONFIGURATION_SYSTEM_REVIEW.md (latest, 34 KB, comprehensive)
ARCHIVE: CONFIGURATION_ANALYSIS.md (superseded, moved for reference)
ARCHIVE: CONFIGURATION_INDEX.md (superseded, moved for reference)
```

### 10. Metadata Only When Essential

**CONSTRAINT**: Don't document obvious metadata

**INCLUDE**:
```yaml
title: "..."
type: "architecture|guide|reference|..."  # Clarifies intent
status: "active|deprecated|archived"       # Critical for navigation
```

**DON'T INCLUDE in metadata**:
- Descriptions of what metadata is (it's obvious)
- Explanations of why each field exists (document standard once)
- Redundant copies of title/type in body text

### 11. Consolidate, Don't Proliferate

**CONSTRAINT**: When you find multiple documents covering the same topic, consolidate into one authoritative document

**RED FLAGS** (indicates proliferation):
- Topic name + "_SUMMARY.md" + "_REVIEW.md" + "_GUIDE.md" + "_INDEX.md"
- Multiple documents with >50% overlapping content
- "Quick" versions of documents that duplicate the main content
- Visual, text, and summary versions of the same analysis

**CONSOLIDATION RULE**:
- If 3+ documents cover the same topic with >40% overlap: **Consolidate into ONE primary document**
- Archive the others with "SUPERSEDED by X" notices
- Create at most ONE quick reference (if the primary is >30KB)

**EXAMPLE VIOLATION**:
```
VULKAN_ROBUSTNESS_REVIEW.md (17KB)
VULKAN_ROBUSTNESS_SUMMARY.md (8KB) - overlaps 70% with REVIEW
VULKAN_ROBUSTNESS_QUICK_FIX.md (12KB) - overlaps 50% with REVIEW
VULKAN_ROBUSTNESS_INDEX.md (11KB) - overlaps 40% with REVIEW
VULKAN_ROBUSTNESS_VISUAL_GUIDE.md (13KB) - overlaps 60% with REVIEW
→ VIOLATION: 5 documents, 61KB total, massive overlap
```

**CORRECT APPROACH**:
```
VULKAN_ROBUSTNESS_GUIDE.md (25KB) - Comprehensive, includes all info
[Archive the other 4 with consolidation notice]
```

**ACTION REQUIRED**: Before creating a new document, check if similar documents exist. If they do, either:
1. Update the existing document
2. Consolidate multiple docs into one
3. Add a cross-reference instead of duplicating content

---

## Documentation as Code: Role-Based System

This section establishes a role-based documentation framework. Rather than creating new documentation for every change, documentation should be understood as fulfilling specific **roles** in the project. When a project element is updated or upgraded, its corresponding documentation is **updated in-place** rather than creating new documents.

### Core Philosophy

**Documentation fills roles, not timestamps.** A documentation file's purpose is to fulfill a specific role in describing/explaining/guiding the project. When source code changes, documentation for that code is updated. When a system upgrades, its documentation is upgraded. Documentation is not a snapshot in time—it is a living description of the current state.

**New documentation is created only when a new role needs filling.** Before creating new documentation, ask: "Does an existing document already fill this role? If so, update it. If not, what new role needs to be filled?"

### Defined Documentation Roles

A role describes the **purpose and audience** of a document. Documents should be categorized by role:

#### Role 1: Onboarding & Entry Point
**Purpose**: Help new developers/users understand what the project is and get started quickly
**Audience**: New contributors, external users
**Update Trigger**: When getting started becomes harder/easier, when prerequisites change
**Examples**: START_HERE.md, SETUP.md, README.md
**Longevity**: Long-lived; core role that persists across versions

#### Role 2: Architecture & Design Reference
**Purpose**: Explain how core systems work, data flows, design decisions
**Audience**: Developers working on system internals
**Update Trigger**: When architecture changes, when new subsystems are added, when design decisions evolve
**Examples**: IMPLEMENTATION_SUMMARY.md, CONFIGURATION_SYSTEM_REVIEW.md, SEMANTIC_ASSET_HASHING.md
**Longevity**: Long-lived with periodic updates; may evolve significantly

#### Role 3: Quick Reference & Lookup
**Purpose**: Provide fast, focused reference information without context or prose
**Audience**: Developers actively using the system
**Update Trigger**: When API changes, when configuration options change, when parameters evolve
**Examples**: CONFIGURATION_QUICK_REFERENCE.md, API reference documents
**Longevity**: Frequently updated; should stay current without archive

#### Role 4: Component-Specific Deep Dive
**Purpose**: Provide comprehensive analysis of a specific component or subsystem
**Audience**: Developers working on that component
**Update Trigger**: When component is analyzed, refactored, or upgraded
**Examples**: CONFIGURATOR_ANALYSIS_GUIDE.md
**Longevity**: Long-lived; replaces older analysis with new version

#### Role 5: Decision Record
**Purpose**: Record important technical decisions and their rationale
**Audience**: Developers understanding why things are done a certain way
**Update Trigger**: When new decisions are made; older decisions don't require updates (they're historical)
**Examples**: SUBMODULE_STRATEGY.md, architecture decision records
**Longevity**: Permanent; decisions don't change, new decisions create new records

#### Role 6: Integration & Workflow Guide
**Purpose**: Explain end-to-end processes, integrations, and cross-system workflows
**Audience**: Developers building integrations or working across systems
**Update Trigger**: When integrations change, when workflows are redesigned
**Examples**: INTEGRATION_GUIDE.md, CLAUDE_CODE_INTEGRATION_GUIDE.md
**Longevity**: Long-lived; updated when workflows change

#### Role 7: Status & Summary (Temporary - USE SPARINGLY)
**Purpose**: Capture current state snapshot for a specific point in time
**Audience**: Project stakeholders, historical record
**Update Trigger**: This role should be **filled temporarily**; content becomes stale
**Examples**: DOCUMENTATION_CLEANUP_SUMMARY.md, phase completion summaries, audit reports
**Longevity**: Short-lived; archive after replacing with updated status or when superseded

**IMPORTANT**: This role should be filled ONLY when explicitly requested by the user. Do NOT automatically generate status/summary documents after completing work. Status information belongs in git commits and chat responses, not in proliferating markdown files.

### Role-Based Update Strategy

**When something in the project changes:**

1. **Identify which role(s) are affected**
   - Example: "We refactored the configuration system"
   - Affected roles: Architecture & Design Reference, Quick Reference, Component Deep Dive

2. **Update the documents filling those roles**
   - Architecture doc: Explain new architecture
   - Quick reference: Update API/parameters
   - Component guide: Provide new analysis
   - Do NOT create new documents to duplicate this information

3. **Archive only what's superseded**
   - If a new version of a role-filling document replaces an old one, archive the old version
   - Don't archive just because content changed—only archive if the document is superseded entirely

4. **Never create documents for stale roles**
   - If documentation is a "status snapshot," update it in-place or archive it when stale
   - Don't keep multiple status documents (2025-10-31 version, 2025-11-15 version, etc.)

### Role-Based Decision Matrix

| Scenario | Decision | Action |
|----------|----------|--------|
| Code changes, documentation exists for that code | Update in place | Edit the existing doc to reflect changes |
| New feature added, no doc covers it | New role needs filling | Create new documentation for the new feature |
| Architecture redesigned | Update existing architecture docs | Replace/update IMPLEMENTATION_SUMMARY.md and related docs |
| System API changes | Update quick reference | Edit CONFIGURATION_QUICK_REFERENCE.md with new params |
| Phase completed, status captured | Document is temporary | Decide: update status or archive old status doc |
| Multiple docs describe same system | Redundant roles | Keep one authoritative doc (role), archive others |
| Document becomes stale/unmaintainable | Role not being filled | Archive the doc; create new one or leave role unfilled |

### Practical Examples

**Example 1: Configuration System Changes**

Status: Configuration system refactored (new validation, new parameters)

Affected Roles:
- Architecture & Design Reference → `CONFIGURATION_SYSTEM_REVIEW.md`
- Quick Reference → `CONFIGURATION_QUICK_REFERENCE.md`

Action:
1. Update `CONFIGURATION_SYSTEM_REVIEW.md` with new architecture details
2. Update `CONFIGURATION_QUICK_REFERENCE.md` with new parameters
3. Do NOT create `CONFIGURATION_SYSTEM_REVIEW_2025.md` or new versions
4. Old versions go to ARCHIVE only if completely replaced (not if just updated)

**Example 2: Status Documentation**

Created: `DOCUMENTATION_CLEANUP_SUMMARY.md` (2025-10-31)

Status: 2 weeks later, new files added and need to update summary

Options:
- Option A: Update `DOCUMENTATION_CLEANUP_SUMMARY.md` with latest status
- Option B: Archive it and create new summary if this role needs a fresh perspective
- Option C: Stop maintaining it (don't update stale status docs)

Choose based on: Is the status role still important? If yes, keep one current version. If no, archive and let role go unfilled.

**Example 3: Component Analysis**

Created: `CONFIGURATOR_ANALYSIS_GUIDE.md` (detailed analysis)

Status: Configurator significantly refactored

Action:
1. Update `CONFIGURATOR_ANALYSIS_GUIDE.md` with new analysis (same role, new content)
2. If old analysis is valuable for history, note changes at top: "Updated 2025-11-15 with current architecture"
3. Don't create `CONFIGURATOR_ANALYSIS_GUIDE_CURRENT.md`

### When Role Filling Fails

Sometimes documentation roles go unfilled:

- **Acceptable**: "We don't currently document the build system" (role exists, document doesn't)
- **Unacceptable**: "We have 3 conflicting build system docs" (multiple docs filling one role)
- **Action**: Archive extras, keep one authoritative doc per role

### Key Principles

1. **Roles are persistent; documents are mutable.** The role of "explaining configuration" persists; the document explaining it may be updated many times.

2. **One authoritative document per role.** You may have multiple tiers (quick reference + deep dive), but they're filling different roles.

3. **Update in place, don't proliferate.** When code changes, update its docs. Don't create new docs about the same thing.

4. **Status documents are temporary.** If a doc captures "state at time X," it has short lifespan. Either keep it current or archive it.

5. **Archive only when replacing.** Don't archive docs just because content changed—only when document itself is superseded by a different document.

---

## Documentation Generation Guidelines

When creating documentation for DevMe:

### 1. Start with Necessity

**Question**: "Is this documentation necessary or can it be eliminated?"

**Good reasons to document**:
- Explains how to use a tool
- Describes system architecture
- Records important decisions
- Provides reference data
- Guides new contributors

**Bad reasons (don't document)**:
- "We might need this someday" (document when needed)
- "This is how we did it once" (unless historically important)
- "It's a good summary of X" (link to X instead)
- "This explains the obvious" (your code/comments should be clear)

### 2. Structure for Discovery

Document structure should enable:
- **Quick start**: New user can get started
- **Key concepts**: Core architecture understanding
- **Deep dive**: Detailed technical info
- **Reference**: Facts and data

**Use clear headers and progression**.

### 3. Task and Phase Referencing

**CONSTRAINT**: When documenting tasks, todos, or multi-step processes, use phase-based numbering instead of time estimates.

**Phase-Based Numbering Format**:
```
Phase 1: Overall objective
  - Phase 1.1: First subtask
  - Phase 1.2: Second subtask
  - Phase 1.3: Third subtask

Phase 2: Next objective
  - Phase 2.1: First subtask
  - Phase 2.2: Second subtask
```

**Examples**:

❌ **BAD** (time estimates):
```markdown
## Implementation Plan
1. Set up infrastructure (2 hours)
2. Implement core features (4 hours)
3. Write tests (1 hour)
4. Deploy (30 minutes)
```

✅ **GOOD** (phase-based):
```markdown
## Implementation Plan

Phase 1: Infrastructure Setup
  - Phase 1.1: Configure build system
  - Phase 1.2: Set up dependencies
  - Phase 1.3: Initialize database schema

Phase 2: Core Features
  - Phase 2.1: Implement authentication
  - Phase 2.2: Build API endpoints
  - Phase 2.3: Add validation layer

Phase 3: Testing & Deployment
  - Phase 3.1: Write unit tests
  - Phase 3.2: Integration testing
  - Phase 3.3: Deploy to staging
```

**Rationale**:
- Time estimates become inaccurate and create false expectations
- Phase numbering shows logical task progression and dependencies
- Easier to reference specific tasks ("completed Phase 2.3")
- Allows flexible subdivision without timeline pressure

**When to use phase-based numbering**:
- Multi-step implementation plans
- Migration guides
- Onboarding checklists
- Development roadmaps
- Task tracking in documentation

### 4. Include Context, Not Duplication

**INSTEAD OF**: "Here's how to use Intent.create()... [20 lines of example]"

**USE**: "Use Intent.create() (see example in CONFIGURATOR_ANALYSIS_GUIDE.md). Key parameters: type, params, meta."

### 5. Version Proactively

When you create Document B that supersedes Document A:
1. Keep Document A in ARCHIVE/
2. Add note: "SUPERSEDED by Document B"
3. Don't force readers to hunt for latest version

### 6. Connect the Dots

Link related documents using `related` field:
```yaml
related: ["SETUP.md", "CONFIGURATION_SYSTEM_DESIGN.md", "BLENDER_INTEGRATION_GUIDE.md"]
```

Make finding related documentation trivial.

---

## Documentation Audit Findings (Baseline)

**Current State** (2025-10-31):
- 167 files, 2.3 MB total
- 25% redundant/duplicate content (290 KB waste)
- 91% missing standardized metadata
- Poor cross-referencing

**Target State** (Post-Phase 2):
- 158 files (remove duplicates)
- 2.0 MB total (280K freed)
- 100% standardized metadata
- Full cross-reference mapping
- Zero duplicate content

**Key Files for Reference**:
- DOCUMENTATION_AUDIT_REPORT.md - Detailed findings
- DOCUMENTATION_SYSTEM_ROADMAP.md - 4-phase cleanup plan
- DOCUMENTATION_MANAGEMENT_SYSTEM.md - Technical standard

---

## Documentation Workflow

### Creating New Documentation

1. **Check if it already exists**
   - Search for similar documents
   - Ask: "Is there already a doc covering this?"
   - If yes: Add to `related` field, don't duplicate

2. **Determine documentation type**
   - guide (how-to)
   - architecture (system design)
   - reference (API specs, lookup)
   - analysis (research, exploration)
   - other (see taxonomy)

3. **Write to appropriate tier**
   - Tier 1: Primary, comprehensive (write once)
   - Tier 2: Quick reference (link to Tier 1)
   - Tier 3: Deep dives (specialized topics)

4. **Include required metadata**
   ```yaml
   ---
   title: "Descriptive Title"
   type: "guide|architecture|reference|analysis|..."
   topics: ["topic1", "topic2"]
   author: "Your Name"
   created: "YYYY-MM-DD"
   status: "active"
   related: ["RelatedDoc.md"]
   ---
   ```

5. **Link to existing docs**
   - Don't explain concepts documented elsewhere
   - Use "See [doc name](path) for details"
   - Keep your doc focused on new material

### Superseding Existing Documentation

1. **Create new version** with latest info
2. **Move old version** to ARCHIVE/
3. **Add note** to archived version pointing to new doc
4. **Update all `related` links** to point to new version
5. **Git commit** with clear message: "Supersede OLD_DOC.md with NEW_DOC.md"

### Archiving Documents

When document is no longer current:
1. Move to `/home/kelly/devMe/ARCHIVE/`
2. Add header: `ARCHIVED: See [current doc](../CURRENT_DOC.md)`
3. Keep git history (don't delete permanently)
4. Remove from active cross-references

---

## Quick Checklist Before Submitting Documentation

- [ ] **Unique value**: Does this add something not elsewhere?
- [ ] **Necessary**: Would the project suffer without it?
- [ ] **Concise**: No boilerplate sections?
- [ ] **Linked**: Cross-references to related docs included?
- [ ] **Structured**: Tables/lists instead of prose when possible?
- [ ] **Sized**: Within appropriate size limits for type?
- [ ] **Metadata**: YAML frontmatter complete?
- [ ] **No duplicates**: Searched for existing similar docs?
- [ ] **Discoverable**: Would a new contributor find it?

---

## Configuration Notes

This CLAUDE.md is loaded automatically by Claude Code and influences code generation behavior.

**See also**:
- `.claude.json` - Tool and permission settings
- `/home/kelly/.claude/settings.json` - User preferences
- `/home/kelly/devMe/.devme.json` - Project-specific config

---

---

## Amendment: Documentation as Code (Role-Based System)

**Added**: 2025-10-31 (amendment)
**Rationale**: To establish a sustainable documentation practice that focuses on maintaining documentation as living code that describes current system state, rather than creating new documents for every change.

**Key Change**: Documentation is now understood through **roles** (onboarding, architecture reference, quick lookup, etc.) rather than timestamps. Updates to the project should trigger updates to role-filling documents, not creation of new documents.

**Impact**:
- Reduces documentation sprawl (no need for "2025 version" of architecture docs)
- Makes documentation maintenance sustainable (update in place rather than version)
- Clarifies decision: update existing doc vs. create new doc vs. let role go unfilled
- Enables "documentation as code" philosophy where docs stay current with implementation

See **"Documentation as Code: Role-Based System"** section for complete framework.

---

**Last Updated**: 2025-10-31 (amendment 2025-10-31)
**Purpose**: Enforce concise, non-redundant documentation practices + establish role-based documentation system
**Maintainer**: Team
**Status**: Active guidance

For questions about documentation standards, see DOCUMENTATION_MANAGEMENT_SYSTEM.md (archived) or review role-based system section above
