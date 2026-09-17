#!/usr/bin/env python3
"""
Generate a streaming synthetic Claude Code + Codex corpus for scale testing.

This writes a realistic-looking Claude Code `projects/` tree and a Codex `sessions/`
tree into an output directory, without reading any real agent logs. It exists so
engineers compacting urollup's in-memory data model can measure how memory and time
scale with input size on a reproducible, privacy-safe corpus instead of anyone's real
history (see docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md, Phase 2).

Records are modeled on the shapes documented in
crates/urollup-core/tests/fixtures/{claude-project,codex-rollout}/*/README.md: Claude
assistant records with usage and repeated block records per `message.id`, subagents with
`.meta.json` sidecars, resumed sessions that replay another session's `uuid` and
`sessionId`, `progress` records nesting a subagent's message, and `quotaLimits`; Codex
`session_meta`/`turn_context`/`event_msg` `token_count` records with `rate_limits`,
`token_usage_record` rollouts, and forked rollouts with copied history.

Generation is a streaming, bounded-memory walk: each session or rollout ("unit") is
built as a small in-memory batch of JSON lines, sized, and only committed to disk if it
fits the remaining `--max-bytes` budget. Nothing holds the whole corpus in memory at
once, and everything is deterministic from `--seed`: the same seed and options always
produce byte-identical output.

Run with `uv --config-file uv.toml run --frozen python scripts/generate-synthetic-corpus.py`.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import random
import shutil
import subprocess
import sys
import uuid
from collections import deque
from dataclasses import dataclass, field
from datetime import date, datetime, timedelta, timezone
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Deterministic identifiers and text
# ---------------------------------------------------------------------------

# A fixed namespace so `uuid.uuid5` derivations are stable across runs and machines.
_ID_NAMESPACE = uuid.UUID("2f6a8b8e-8c1a-4a9b-9f0e-6d6a1a2b3c4d")

CLAUDE_VERSION = "2.1.214"
CODEX_CLI_VERSION = "0.154.0"
CLAUDE_MODELS = ["claude-sonnet-4-5", "claude-haiku-4-5", "claude-opus-4-5"]
CODEX_MODELS = ["gpt-5.2-codex", "gpt-5.1-codex", "gpt-5.2-codex-mini"]
CODEX_EFFORTS = ["low", "medium", "high"]
NON_SPAWN_TOOLS = ["Bash", "Read", "Grep", "Edit", "Write"]
MAX_RECORD_PADDING_BYTES = 32 * 1024 * 1024  # keeps single JSON lines well under the
# engine's 64 MiB per-record ceiling (crates/urollup-core/src/sources/reader.rs).

# Defaults for --days/--claude-sessions-per-day/--codex-rollouts-per-day, when the
# caller leaves them unset, are derived so --max-bytes stays the binding limit rather
# than running out of scheduled sessions/rollouts first. A no-padding unit is at least a
# couple KB (a session or rollout with a handful of turns); this floor is deliberately
# conservative (smaller than any observed unit) and then quadrupled for headroom, so the
# derived count ceiling comfortably exceeds what a real run needs.
DEFAULT_DAYS = 180
MIN_UNIT_BYTES_ESTIMATE = 2000
COUNT_HEADROOM = 4

_FILLER_UNIT = "synthetic filler content for scale testing padding purposes "


def det_uuid(seed: int, *parts: object) -> str:
    """A deterministic, seed-scoped UUID string in standard 36-character form."""
    name = f"{seed}:" + ":".join(str(part) for part in parts)
    return str(uuid.uuid5(_ID_NAMESPACE, name))


def det_hex(seed: int, *parts: object, length: int = 16) -> str:
    """A deterministic lowercase hex string of `length` characters."""
    name = f"{seed}:" + ":".join(str(part) for part in parts)
    digest = hashlib.sha256(name.encode("utf-8")).hexdigest()
    while len(digest) < length:
        digest += hashlib.sha256(digest.encode("utf-8")).hexdigest()
    return digest[:length]


def filler(padding_bytes: int) -> str:
    """`padding_bytes` of deterministic, plain-ASCII filler text (or "" for 0)."""
    if padding_bytes <= 0:
        return ""
    reps = padding_bytes // len(_FILLER_UNIT) + 1
    return (_FILLER_UNIT * reps)[:padding_bytes]


def padded(base: str, padding_bytes: int) -> str:
    """`base` text with `padding_bytes` of extra content appended.

    Used only on fields the accounting engine never reads (prompts, replies, tool
    descriptions), so raw byte growth here never changes a request's usage numbers.
    """
    extra = filler(padding_bytes)
    return f"{base} {extra}" if extra else base


# ---------------------------------------------------------------------------
# Time
# ---------------------------------------------------------------------------

# A fixed synthetic epoch, not wall-clock "today": determinism from `--seed` requires
# that nothing here depends on when the generator runs.
BASE_DATE = date(2026, 1, 5)


def day_date(day_index: int) -> date:
    return BASE_DATE + timedelta(days=day_index)


class Clock:
    """A small, seeded, monotonically advancing UTC clock for one unit's records."""

    def __init__(self, start: datetime, rng: random.Random) -> None:
        self.dt = start
        self.rng = rng

    def tick(self, lo: int = 1, hi: int = 90) -> str:
        self.dt += timedelta(seconds=self.rng.randint(lo, hi))
        return self.iso()

    def tick_ms(self, ms: int) -> str:
        self.dt += timedelta(milliseconds=ms)
        return self.iso()

    def iso(self) -> str:
        millis = self.dt.microsecond // 1000
        return self.dt.strftime("%Y-%m-%dT%H:%M:%S.") + f"{millis:03d}Z"

    def epoch(self) -> int:
        return int(self.dt.timestamp())


def start_of_day(day_index: int, hour: int, minute: int, second: int) -> datetime:
    d = day_date(day_index)
    return datetime(d.year, d.month, d.day, hour, minute, second, tzinfo=timezone.utc)


# ---------------------------------------------------------------------------
# Byte budget and unit staging
# ---------------------------------------------------------------------------


class ByteBudget:
    """A byte budget that a unit is committed against only if it still fits.

    `limit=None` means unlimited (bounded only by explicit session/day counts), which
    the raw-bytes-independence measurement mode relies on to keep usage records
    identical while content padding varies.
    """

    def __init__(self, limit: int | None) -> None:
        self.limit = limit
        self.used = 0

    def fits(self, extra: int) -> bool:
        return self.limit is None or self.used + extra <= self.limit

    def add(self, extra: int) -> None:
        self.used += extra

    def exhausted(self) -> bool:
        return self.limit is not None and self.used >= self.limit


@dataclass
class Unit:
    """One session or rollout, staged in memory before it is sized and committed.

    Kept small (a handful of files, each a few dozen lines) so memory never scales with
    the corpus total: only one unit is ever staged at a time.
    """

    files: dict[str, list[str]] = field(default_factory=dict)
    zstd_eligible: list[str] = field(default_factory=list)
    pending: dict[str, int] = field(default_factory=dict)

    def bump(self, key: str, amount: int = 1) -> None:
        """Stages a counter increment that only counts if this unit is committed.

        Builders call this instead of touching `RunContext.counters` directly, so a
        unit discarded for not fitting the remaining byte budget never leaves phantom
        counts (or phantom pool entries referencing content that was never written).
        """
        self.pending[key] = self.pending.get(key, 0) + amount

    def add_line(self, rel_path: str, record: dict[str, Any]) -> None:
        self.files.setdefault(rel_path, []).append(json.dumps(record, separators=(",", ":")))

    def byte_size(self) -> int:
        total = 0
        for lines in self.files.values():
            for line in lines:
                total += len(line.encode("utf-8")) + 1  # +1 for the trailing newline
        return total


# zstd is spawned as a subprocess (the option only works with a CLI binary on PATH, not
# a library), and per-file invocations dominate generation time once a corpus has
# thousands of rollouts. Batching many paths into one invocation (zstd compresses each
# argument to its own `.zst` file) cuts subprocess-start overhead by orders of magnitude.
ZSTD_BATCH_SIZE = 500


def commit_unit(
    root: Path,
    unit: Unit,
    budget: ByteBudget,
    zstd_fraction: float,
    rng: random.Random,
    counters: Counters,
    zstd_selected: list[Path],
) -> bool:
    """Writes `unit` under `root` if it fits `budget`, then returns whether it did.

    Sizing happens before any file is opened, so a unit that would overshoot the cap is
    never partially written: the final corpus is always at or under `--max-bytes`. Only
    a successful commit merges the unit's staged counters (`Unit.bump`); a caller must
    likewise only register this unit in a resume/fork pool when this returns `True`, so
    a discarded unit never leaves phantom counts or dangling cross-references.

    A file chosen for zstd (by the same per-file `zstd_fraction` draw as before) is
    appended to `zstd_selected` rather than compressed immediately: `generate` batches
    the whole run's selections into a handful of `zstd` invocations at the end.
    """
    size = unit.byte_size()
    if not budget.fits(size):
        return False
    for rel_path, lines in unit.files.items():
        full = root / rel_path
        full.parent.mkdir(parents=True, exist_ok=True)
        with full.open("w", encoding="utf-8") as handle:
            for line in lines:
                handle.write(line)
                handle.write("\n")
    budget.add(size)
    counters.merge(unit.pending)
    if zstd_fraction > 0:
        for rel_path in unit.zstd_eligible:
            if rng.random() < zstd_fraction:
                zstd_selected.append(root / rel_path)
    return True


def compress_selected(paths: list[Path]) -> None:
    """Compresses `paths` to `.zst` in place, removing the originals, in batches."""
    for start in range(0, len(paths), ZSTD_BATCH_SIZE):
        batch = paths[start : start + ZSTD_BATCH_SIZE]
        subprocess.run(
            ["zstd", "-q", "-f", "--rm", *(str(path) for path in batch)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )


# ---------------------------------------------------------------------------
# Shared generation context
# ---------------------------------------------------------------------------


@dataclass
class Options:
    seed: int
    max_bytes: int | None
    claude_fraction: float
    content_padding_bytes: int
    zstd_fraction: float
    resume_fraction: float
    subagent_fraction: float
    workflow_fraction: float
    quota_fraction: float
    fork_fraction: float
    token_usage_record_fraction: float
    claude_projects: int
    days: int
    claude_sessions_per_day: int
    codex_rollouts_per_day: int


@dataclass
class Counters:
    claude_sessions: int = 0
    claude_resumed_sessions: int = 0
    claude_subagent_files: int = 0
    claude_workflow_subagent_files: int = 0
    claude_files: int = 0
    codex_rollouts: int = 0
    codex_forked_rollouts: int = 0
    codex_token_usage_record_rollouts: int = 0
    codex_zstd_rollouts: int = 0
    codex_files: int = 0
    usage_records: int = 0

    def merge(self, pending: dict[str, int]) -> None:
        """Adds a committed unit's staged counts (see `Unit.bump`) into the totals."""
        for key, amount in pending.items():
            setattr(self, key, getattr(self, key) + amount)


class RunContext:
    """RNG, id counters and bounded resume/fork pools shared across one generation run."""

    def __init__(self, options: Options) -> None:
        self.options = options
        self.rng = random.Random(options.seed)
        self.counters = Counters()
        self._next_id = 0
        # Bounded pools: memory never grows with corpus size, only with pool depth.
        self.claude_pool: deque[dict[str, Any]] = deque(maxlen=40)
        self.codex_pool: deque[dict[str, Any]] = deque(maxlen=40)

    def next_id(self) -> int:
        self._next_id += 1
        return self._next_id


# ---------------------------------------------------------------------------
# Claude Code generation
# ---------------------------------------------------------------------------


def gen_claude_usage(ctx: RunContext) -> dict[str, Any]:
    rng = ctx.rng
    input_tokens = rng.randint(1, 40)
    cache_write = rng.choice([0, 0, rng.randint(50, 2000)])
    cache_read = rng.choice([0, rng.randint(200, 60000)])
    output_tokens = rng.randint(5, 400)
    return {
        "input_tokens": input_tokens,
        "cache_creation_input_tokens": cache_write,
        "cache_read_input_tokens": cache_read,
        "cache_creation": {
            "ephemeral_5m_input_tokens": cache_write,
            "ephemeral_1h_input_tokens": 0,
        },
        "output_tokens": output_tokens,
        "service_tier": "standard",
    }


def gen_quota_limits(ctx: RunContext, clock: Clock) -> dict[str, Any]:
    rng = ctx.rng
    status = rng.choice(["allowed", "allowed_warning", "rejected"])
    return {
        "status": status,
        "rateLimitType": rng.choice(["five_hour", "seven_day"]),
        "resetsAt": clock.epoch() + rng.randint(3600, 7 * 24 * 3600),
        "isUsingOverage": rng.random() < 0.1,
        "overageStatus": "rejected" if status == "rejected" else "allowed",
        "overageDisabledReason": "out_of_credits" if status == "rejected" else None,
        "unifiedRateLimitFallbackAvailable": rng.random() < 0.3,
    }


def envelope(session_id: str, cwd: str, is_sidechain: bool, agent_id: str | None) -> dict[str, Any]:
    record: dict[str, Any] = {
        "parentUuid": None,
        "isSidechain": is_sidechain,
        "userType": "external",
        "cwd": cwd,
        "sessionId": session_id,
        "version": CLAUDE_VERSION,
        "gitBranch": "main",
    }
    if agent_id is not None:
        record["agentId"] = agent_id
    return record


def build_assistant_block_group(
    ctx: RunContext,
    unit: Unit,
    session_id: str,
    cwd: str,
    is_sidechain: bool,
    agent_id: str | None,
    parent_uuid: str | None,
    clock: Clock,
    spawn_tool: str | None,
    padding: int,
    quota_fraction: float,
) -> tuple[list[dict[str, Any]], str, str | None]:
    """Builds 1-3 block records sharing one `message.id` (design's block-record rule).

    Returns the record dicts, the last record's uuid, and the spawning tool_use id when
    `spawn_tool` requested one (e.g. "Agent").
    """
    rng = ctx.rng
    msg_id = "msg_" + det_hex(ctx.options.seed, "claude-msg", ctx.next_id())
    req_id = "req_" + det_hex(ctx.options.seed, "claude-req", ctx.next_id())
    model = rng.choice(CLAUDE_MODELS)
    roll = rng.random()
    n_blocks = 3 if roll < 0.05 else 2 if roll < 0.30 else 1
    base_usage = gen_claude_usage(ctx)
    records: list[dict[str, Any]] = []
    tool_use_id: str | None = None
    last_uuid = parent_uuid
    for index in range(n_blocks):
        is_last = index == n_blocks - 1
        usage = dict(base_usage)
        usage["cache_creation"] = dict(base_usage["cache_creation"])
        usage["output_tokens"] = base_usage["output_tokens"] + index * rng.randint(5, 60)
        if is_last and spawn_tool is not None:
            tool_use_id = "toolu_" + det_hex(ctx.options.seed, "claude-tool", ctx.next_id())
            content: list[dict[str, Any]] = [
                {
                    "type": "tool_use",
                    "id": tool_use_id,
                    "name": spawn_tool,
                    "input": {
                        "description": padded("Synthetic task.", padding),
                        "prompt": padded("Synthetic prompt.", padding),
                        "subagent_type": "general-purpose"
                        if spawn_tool == "Agent"
                        else "workflow-subagent",
                    },
                }
            ]
            stop_reason: str | None = "tool_use"
        elif is_last and rng.random() < 0.35:
            tool_use_id = "toolu_" + det_hex(ctx.options.seed, "claude-tool", ctx.next_id())
            tool_name = rng.choice(NON_SPAWN_TOOLS)
            content = [
                {
                    "type": "tool_use",
                    "id": tool_use_id,
                    "name": tool_name,
                    "input": {"command": padded("cargo test", padding)}
                    if tool_name == "Bash"
                    else {"pattern": padded("synthetic", padding)},
                }
            ]
            stop_reason = "tool_use"
        elif is_last:
            content = [{"type": "text", "text": padded("Synthetic reply.", padding)}]
            stop_reason = "end_turn"
        elif rng.random() < 0.5:
            content = [
                {
                    "type": "thinking",
                    "thinking": padded("Synthetic reasoning.", padding),
                    "signature": "c3ludGhldGlj",
                }
            ]
            stop_reason = None
        else:
            content = [{"type": "text", "text": padded("Synthetic partial reply.", padding)}]
            stop_reason = None
        record = envelope(session_id, cwd, is_sidechain, agent_id)
        record["message"] = {
            "id": msg_id,
            "type": "message",
            "role": "assistant",
            "model": model,
            "content": content,
            "stop_reason": stop_reason,
            "stop_sequence": None,
            "usage": usage,
        }
        record["requestId"] = req_id
        record["type"] = "assistant"
        record["parentUuid"] = last_uuid
        record["uuid"] = det_uuid(ctx.options.seed, "claude-uuid", ctx.next_id())
        record["timestamp"] = clock.tick(1, 4) if index else clock.tick(1, 3)
        if rng.random() < quota_fraction:
            record["quotaLimits"] = gen_quota_limits(ctx, clock)
        records.append(record)
        last_uuid = record["uuid"]
        unit.bump("usage_records")
    return records, last_uuid, tool_use_id


def build_user_record(
    session_id: str,
    cwd: str,
    is_sidechain: bool,
    agent_id: str | None,
    parent_uuid: str | None,
    clock: Clock,
    seed: int,
    counter: int,
    content: Any,
    tool_result: dict[str, Any] | None = None,
) -> dict[str, Any]:
    record = envelope(session_id, cwd, is_sidechain, agent_id)
    record["type"] = "user"
    record["message"] = {"role": "user", "content": content}
    record["parentUuid"] = parent_uuid
    record["uuid"] = det_uuid(seed, "claude-uuid", counter)
    record["timestamp"] = clock.tick(2, 120)
    if tool_result is not None:
        record["toolUseResult"] = tool_result
    return record


def build_claude_thread(
    ctx: RunContext,
    session_id: str,
    cwd: str,
    is_sidechain: bool,
    agent_id: str | None,
    clock: Clock,
    n_turns: int,
    padding: int,
    allow_spawn: bool,
    unit: Unit,
    project_dir: str,
    depth: int,
) -> list[dict[str, Any]]:
    """Builds a linear turn sequence (subagent or top-level), spawning children inline.

    Returns the flat list of record dicts written for this thread's own file, so a
    caller can copy them verbatim into a resumed session or a nested progress record.
    """
    rng = ctx.rng
    records: list[dict[str, Any]] = []
    parent_uuid: str | None = None
    for turn in range(n_turns):
        prompt = "Synthetic task." if turn == 0 and is_sidechain else "Synthetic prompt."
        if turn > 0:
            prompt = "Synthetic follow-up."
        user_record = build_user_record(
            session_id,
            cwd,
            is_sidechain,
            agent_id,
            parent_uuid,
            clock,
            ctx.options.seed,
            ctx.next_id(),
            padded(prompt, padding),
        )
        records.append(user_record)
        parent_uuid = user_record["uuid"]

        spawn_child = (
            allow_spawn
            and depth < 2
            and turn == n_turns // 2
            and rng.random() < ctx.options.subagent_fraction
        )
        spawn_tool = "Agent" if spawn_child else None
        blocks, parent_uuid, tool_use_id = build_assistant_block_group(
            ctx,
            unit,
            session_id,
            cwd,
            is_sidechain,
            agent_id,
            parent_uuid,
            clock,
            spawn_tool,
            padding,
            ctx.options.quota_fraction,
        )
        records.extend(blocks)

        if spawn_child and tool_use_id is not None:
            child_agent_id = "a" + det_hex(ctx.options.seed, "agent-id", ctx.next_id(), length=17)
            is_workflow = depth == 0 and rng.random() < ctx.options.workflow_fraction
            if is_workflow:
                workflow_name = "wf_synthetic_" + det_hex(
                    ctx.options.seed, "workflow", ctx.next_id(), length=4
                )
                child_rel_dir = f"{project_dir}/{session_id}/subagents/workflows/{workflow_name}"
                spawn_depth = 2
                agent_type = "workflow-subagent"
            else:
                child_rel_dir = f"{project_dir}/{session_id}/subagents"
                spawn_depth = depth + 1
                agent_type = "general-purpose"
            child_clock = Clock(clock.dt, rng)
            child_records = build_claude_thread(
                ctx,
                session_id,
                cwd,
                True,
                child_agent_id,
                child_clock,
                n_turns=rng.randint(1, 3),
                padding=padding,
                allow_spawn=allow_spawn,
                unit=unit,
                project_dir=project_dir,
                depth=depth + 1,
            )
            unit.files[f"{child_rel_dir}/agent-{child_agent_id}.jsonl"] = [
                json.dumps(r, separators=(",", ":")) for r in child_records
            ]
            unit.files[f"{child_rel_dir}/agent-{child_agent_id}.meta.json"] = [
                json.dumps(
                    {
                        "agentType": agent_type,
                        "description": padded("Synthetic task.", padding),
                        "toolUseId": tool_use_id,
                        "spawnDepth": spawn_depth,
                    },
                    separators=(",", ":"),
                )
            ]
            if is_workflow:
                unit.bump("claude_workflow_subagent_files")
            else:
                unit.bump("claude_subagent_files")

            if child_records and rng.random() < 0.7:
                # A copy: it reuses the subagent's own message/requestId/uuid, so the
                # accounting engine treats it as extra evidence for that request rather
                # than a second one (design's nested-copy rule; see the
                # progress-nested-subagent fixture).
                first_assistant = next(
                    (r for r in child_records if r.get("type") == "assistant"), None
                )
                if first_assistant is not None:
                    progress = envelope(session_id, cwd, is_sidechain, agent_id)
                    progress["type"] = "progress"
                    progress["parentUuid"] = parent_uuid
                    progress["data"] = {
                        "message": {
                            "type": "assistant",
                            "timestamp": first_assistant["timestamp"],
                            "message": first_assistant["message"],
                            "requestId": first_assistant.get("requestId"),
                            "uuid": first_assistant["uuid"],
                            "isSidechain": True,
                        }
                    }
                    progress["uuid"] = det_uuid(ctx.options.seed, "claude-uuid", ctx.next_id())
                    progress["timestamp"] = clock.tick_ms(1)
                    records.append(progress)
                    unit.bump("usage_records")

            tool_result_record = build_user_record(
                session_id,
                cwd,
                is_sidechain,
                agent_id,
                parent_uuid,
                clock,
                ctx.options.seed,
                ctx.next_id(),
                [
                    {
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": padded("Synthetic tool output.", padding),
                    }
                ],
                tool_result={"status": "completed", "agentId": child_agent_id},
            )
            records.append(tool_result_record)
            parent_uuid = tool_result_record["uuid"]
        elif tool_use_id is not None:
            tool_result_record = build_user_record(
                session_id,
                cwd,
                is_sidechain,
                agent_id,
                parent_uuid,
                clock,
                ctx.options.seed,
                ctx.next_id(),
                [
                    {
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": padded("Synthetic tool output.", padding),
                    }
                ],
            )
            records.append(tool_result_record)
            parent_uuid = tool_result_record["uuid"]

        if rng.random() < 0.04:
            summary = {
                "type": "summary",
                "summary": padded("Synthetic summary.", padding),
                "leafUuid": det_uuid(ctx.options.seed, "claude-uuid", ctx.next_id()),
            }
            records.append(summary)

    if not is_sidechain and rng.random() < ctx.options.quota_fraction / 3:
        error_clock_ts = clock.tick(2, 60)
        epoch = clock.epoch()
        error_record = envelope(session_id, cwd, is_sidechain, agent_id)
        error_record["message"] = {
            "id": det_uuid(ctx.options.seed, "claude-error", ctx.next_id()),
            "type": "message",
            "role": "assistant",
            "model": "<synthetic>",
            "content": [{"type": "text", "text": f"Claude AI usage limit reached|{epoch}"}],
            "stop_reason": "stop_sequence",
            "stop_sequence": None,
            "usage": {
                "input_tokens": 0,
                "output_tokens": 0,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
            },
        }
        error_record["type"] = "assistant"
        error_record["parentUuid"] = parent_uuid
        error_record["uuid"] = det_uuid(ctx.options.seed, "claude-uuid", ctx.next_id())
        error_record["timestamp"] = error_clock_ts
        error_record["isApiErrorMessage"] = True
        records.append(error_record)

    return records


def build_claude_session(
    ctx: RunContext,
    day_index: int,
    session_index: int,
    project_index: int,
) -> tuple[Unit, dict[str, Any]]:
    """Builds one session's `Unit` plus its resume-pool entry.

    The caller registers the pool entry only if the unit is actually committed, so a
    session discarded for exceeding the byte budget can never be "resumed" later by a
    session that copies uuids and a sessionId with no backing file on disk.
    """
    options = ctx.options
    rng = ctx.rng
    project_dir_name = f"-home-synth-project-{project_index:04d}"
    cwd = f"/home/synth/project-{project_index:04d}"
    session_id = det_uuid(options.seed, "claude-session", day_index, session_index)
    unit = Unit()

    resume_source = None
    if ctx.claude_pool and rng.random() < options.resume_fraction:
        resume_source = rng.choice(list(ctx.claude_pool))

    start = start_of_day(day_index, 8, 0, 0) + timedelta(
        minutes=rng.randint(0, 600), seconds=rng.randint(0, 59)
    )
    clock = Clock(start, rng)
    padding = options.content_padding_bytes
    n_turns = rng.randint(2, 6)

    records: list[dict[str, Any]] = []
    resumed_prefix_len = 0
    if resume_source is not None:
        # Only real conversation lines carry a "uuid" to chain from; a "summary" line
        # (see build_claude_thread) does not, so it is never a resumable boundary.
        original_lines = [r for r in resume_source["records"] if "uuid" in r]
        resumed_prefix_len = min(len(original_lines), rng.choice([2, 2, 4]))
        for original in original_lines[:resumed_prefix_len]:
            records.append(dict(original))  # verbatim copy: same sessionId and uuid
        parent_after_resume = records[-1]["uuid"] if records else None
        continuation = build_claude_thread(
            ctx,
            session_id,
            cwd,
            False,
            None,
            clock,
            n_turns=n_turns,
            padding=padding,
            allow_spawn=True,
            unit=unit,
            project_dir=project_dir_name,
            depth=0,
        )
        if continuation:
            continuation[0]["parentUuid"] = parent_after_resume
        records.extend(continuation)
        unit.bump("claude_resumed_sessions")
    else:
        records = build_claude_thread(
            ctx,
            session_id,
            cwd,
            False,
            None,
            clock,
            n_turns=n_turns,
            padding=padding,
            allow_spawn=True,
            unit=unit,
            project_dir=project_dir_name,
            depth=0,
        )

    unit.files[f"{project_dir_name}/{session_id}.jsonl"] = [
        json.dumps(r, separators=(",", ":")) for r in records
    ]
    unit.bump("claude_sessions")
    unit.bump("claude_files", len(unit.files))
    pool_entry = {"session_id": session_id, "records": records}
    return unit, pool_entry


# ---------------------------------------------------------------------------
# Codex generation
# ---------------------------------------------------------------------------


def gen_codex_usage(ctx: RunContext, lo: int = 300, hi: int = 20000) -> dict[str, int]:
    rng = ctx.rng
    input_tokens = rng.randint(lo, hi)
    cached = rng.randint(0, int(input_tokens * 0.85))
    output_tokens = rng.randint(20, max(21, hi // 10))
    reasoning = rng.randint(0, output_tokens // 3)
    return {
        "input_tokens": input_tokens,
        "cached_input_tokens": cached,
        "cache_write_input_tokens": 0,
        "output_tokens": output_tokens,
        "reasoning_output_tokens": reasoning,
        "total_tokens": input_tokens + output_tokens,
    }


def add_usage(a: dict[str, int], b: dict[str, int]) -> dict[str, int]:
    return {key: a[key] + b[key] for key in a}


def gen_rate_limits(ctx: RunContext, clock: Clock, limit_id: str, limit_name: str | None) -> dict[str, Any]:
    rng = ctx.rng
    primary = {
        "used_percent": round(rng.uniform(1, 60), 1),
        "window_minutes": 300,
        "resets_at": clock.epoch() + rng.randint(3600, 18000),
    }
    secondary = None
    if rng.random() < 0.6:
        secondary = {
            "used_percent": round(rng.uniform(1, 20), 1),
            "window_minutes": 10080,
            "resets_at": clock.epoch() + rng.randint(86400, 604800),
        }
    return {
        "limit_id": limit_id,
        "limit_name": limit_name,
        "primary": primary,
        "secondary": secondary,
        "credits": {"has_credits": True, "unlimited": False, "balance": f"{rng.uniform(1, 50):.2f}"},
        "individual_limit": None,
        "spend_control_reached": False,
        "plan_type": rng.choice(["pro", "plus", "team"]),
        "rate_limit_reached_type": None,
    }


def codex_session_meta(
    session_id: str, cwd: str, timestamp: str, seed: int, forked_from: str | None = None
) -> dict[str, Any]:
    payload: dict[str, Any] = {
        "session_id": session_id,
        "id": session_id,
        "timestamp": timestamp,
        "cwd": cwd,
        "originator": "codex_cli_rs",
        "cli_version": CODEX_CLI_VERSION,
        "source": "cli",
        "thread_source": "user",
        "model_provider": "openai",
        "base_instructions": None,
        "history_mode": "legacy",
        "git": {
            "commit_hash": det_hex(seed, "git-commit", session_id, length=40),
            "branch": "main",
            "repository_url": "https://github.com/synth/project.git",
        },
    }
    if forked_from is not None:
        payload["forked_from_id"] = forked_from
    return {"timestamp": timestamp, "type": "session_meta", "payload": payload}


def build_codex_turn(
    ctx: RunContext,
    unit: Unit,
    thread_id: str,
    cwd: str,
    clock: Clock,
    turn_index: int,
    style: str,
    running_total: dict[str, int],
    padding: int,
) -> list[dict[str, Any]]:
    rng = ctx.rng
    records: list[dict[str, Any]] = []
    turn_id = f"turn-{det_hex(ctx.options.seed, 'turn', thread_id, turn_index, length=6)}"
    model = rng.choice(CODEX_MODELS)
    effort = rng.choice(CODEX_EFFORTS)
    records.append(
        {
            "timestamp": clock.iso(),
            "type": "turn_context",
            "payload": {
                "turn_id": turn_id,
                "cwd": cwd,
                "approval_policy": "never",
                "sandbox_policy": {"type": "read-only"},
                "model": model,
                "effort": effort,
                "summary": "auto",
            },
        }
    )
    started_at = clock.epoch()
    records.append(
        {
            "timestamp": clock.tick_ms(10),
            "type": "event_msg",
            "payload": {
                "type": "task_started",
                "turn_id": turn_id,
                "started_at": started_at,
                "model_context_window": 272000,
            },
        }
    )
    records.append(
        {
            "timestamp": clock.tick_ms(10),
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": padded("Synthetic prompt.", padding)}],
            },
        }
    )

    n_responses = rng.randint(1, 3)
    turn_total = {
        "input_tokens": 0,
        "cached_input_tokens": 0,
        "cache_write_input_tokens": 0,
        "output_tokens": 0,
        "reasoning_output_tokens": 0,
        "total_tokens": 0,
    }
    last_usage_record: dict[str, Any] | None = None
    for response_index in range(n_responses):
        usage = gen_codex_usage(ctx)
        turn_total = add_usage(turn_total, usage)
        running_total.update(add_usage(running_total, usage))
        unit.bump("usage_records")
        if style == "modern":
            response_id = "resp_" + det_hex(
                ctx.options.seed, "codex-resp", thread_id, turn_index, response_index, length=12
            )
            payload = {
                "thread_id": thread_id,
                "turn_id": turn_id,
                "session_id": thread_id,
                "root_turn_id": turn_id,
                "response_id": response_id,
                "usage": usage,
                "turn_token_usage": dict(turn_total),
                "thread_token_usage": dict(running_total),
            }
            record = {
                "timestamp": clock.tick(5, 40),
                "type": "token_usage_record",
                "payload": payload,
            }
            records.append(record)
            last_usage_record = payload
            rate_limits = (
                gen_rate_limits(ctx, clock, "codex", None)
                if rng.random() < 0.4
                else None
            )
            records.append(
                {
                    "timestamp": clock.tick_ms(10),
                    "type": "event_msg",
                    "payload": {
                        "type": "token_count",
                        "info": {
                            "total_token_usage": dict(running_total),
                            "last_token_usage": dict(usage),
                            "model_context_window": 272000,
                        },
                        "rate_limits": rate_limits,
                    },
                }
            )
        else:
            records.append(
                {
                    "timestamp": clock.tick(5, 40),
                    "type": "event_msg",
                    "payload": {
                        "type": "token_count",
                        "info": {
                            "total_token_usage": dict(running_total),
                            "last_token_usage": dict(usage),
                            "model_context_window": 272000,
                        },
                        "rate_limits": None,
                    },
                }
            )

    if style == "modern" and rng.random() < 0.08 and last_usage_record is not None:
        records.append(
            {
                "timestamp": clock.tick_ms(100),
                "type": "compacted",
                "payload": {
                    "message": padded("Synthetic compaction summary.", padding),
                    "compaction_response_id": last_usage_record["response_id"],
                    "latest_token_usage_record": dict(last_usage_record),
                },
            }
        )

    if rng.random() < 0.5:
        records.append(
            {
                "timestamp": clock.tick_ms(1),
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {"type": "output_text", "text": padded("Synthetic reply.", padding)}
                    ],
                },
            }
        )

    completed_at = clock.epoch()
    records.append(
        {
            "timestamp": clock.tick(1, 10),
            "type": "event_msg",
            "payload": {
                "type": "task_complete",
                "turn_id": turn_id,
                "last_agent_message": None,
                "started_at": started_at,
                "completed_at": completed_at,
                "duration_ms": (completed_at - started_at) * 1000,
            },
        }
    )
    return records


def build_codex_rollout(
    ctx: RunContext,
    day_index: int,
    rollout_index: int,
) -> tuple[Unit, dict[str, Any]]:
    """Builds one rollout's `Unit` plus its fork-pool entry (see `build_claude_session`)."""
    options = ctx.options
    rng = ctx.rng
    unit = Unit()
    thread_id = det_uuid(options.seed, "codex-rollout", day_index, rollout_index)
    cwd = f"/home/synth/project-{rollout_index % max(1, options.claude_projects):04d}"
    style = "modern" if rng.random() < options.token_usage_record_fraction else "legacy"

    fork_source = None
    if ctx.codex_pool and rng.random() < options.fork_fraction:
        fork_source = rng.choice(list(ctx.codex_pool))

    d = day_date(day_index)
    start_hour = rng.randint(6, 20)
    start = start_of_day(day_index, start_hour, rng.randint(0, 59), rng.randint(0, 59))
    clock = Clock(start, rng)
    file_ts = start.strftime("%Y-%m-%dT%H-%M-%S")
    rel_path = f"sessions/{d.strftime('%Y')}/{d.strftime('%m')}/{d.strftime('%d')}/rollout-{file_ts}-{thread_id}.jsonl"

    records: list[dict[str, Any]] = []
    running_total = {
        "input_tokens": 0,
        "cached_input_tokens": 0,
        "cache_write_input_tokens": 0,
        "output_tokens": 0,
        "reasoning_output_tokens": 0,
        "total_tokens": 0,
    }
    is_fork = fork_source is not None
    if is_fork:
        parent = fork_source
        records.append(codex_session_meta(thread_id, cwd, clock.iso(), options.seed, parent["thread_id"]))
        for original in parent["records"]:
            copied = dict(original)
            copied["timestamp"] = clock.tick_ms(1)
            records.append(copied)
        running_total = dict(parent["running_total"])
        style = parent["style"]
        unit.bump("codex_forked_rollouts")
    else:
        records.append(codex_session_meta(thread_id, cwd, clock.iso(), options.seed))

    padding = options.content_padding_bytes
    n_turns = rng.randint(1, 5)
    for turn_index in range(n_turns):
        if turn_index > 0:
            clock.dt += timedelta(hours=rng.randint(0, 3), minutes=rng.randint(0, 55))
        records.extend(
            build_codex_turn(ctx, unit, thread_id, cwd, clock, turn_index, style, running_total, padding)
        )

    unit.files[rel_path] = [json.dumps(r, separators=(",", ":")) for r in records]
    unit.zstd_eligible.append(rel_path)
    if style == "modern":
        unit.bump("codex_token_usage_record_rollouts")
    unit.bump("codex_rollouts")
    unit.bump("codex_files")
    pool_entry = {
        "thread_id": thread_id,
        "records": records,
        "running_total": dict(running_total),
        "style": style,
    }
    return unit, pool_entry


# ---------------------------------------------------------------------------
# Orchestration
# ---------------------------------------------------------------------------


def generate(options: Options, out_dir: Path) -> dict[str, Any]:
    ctx = RunContext(options)
    claude_root = out_dir / "claude" / "projects"
    codex_root = out_dir / "codex"

    if options.max_bytes is None:
        claude_budget = ByteBudget(None)
        codex_budget = ByteBudget(None)
    else:
        claude_bytes = int(options.max_bytes * options.claude_fraction)
        claude_budget = ByteBudget(claude_bytes)
        codex_budget = ByteBudget(options.max_bytes - claude_bytes)

    # A single oversized unit (e.g. one with many subagents or heavy padding) should not
    # stop generation while a smaller later one might still fit, so failures are
    # tolerated up to a streak; once that many random-sized units in a row all miss,
    # the remaining budget is almost certainly smaller than any plausible unit.
    MAX_CONSECUTIVE_MISSES = 20
    zstd_selected: list[Path] = []

    session_index = 0
    misses = 0
    for day_index in range(options.days):
        if claude_budget.exhausted() or misses >= MAX_CONSECUTIVE_MISSES:
            break
        for _ in range(options.claude_sessions_per_day):
            if claude_budget.exhausted() or misses >= MAX_CONSECUTIVE_MISSES:
                break
            project_index = day_index % max(1, options.claude_projects)
            unit, pool_entry = build_claude_session(ctx, day_index, session_index, project_index)
            session_index += 1
            if commit_unit(claude_root, unit, claude_budget, 0.0, ctx.rng, ctx.counters, zstd_selected):
                ctx.claude_pool.append(pool_entry)
                misses = 0
            else:
                misses += 1

    rollout_index = 0
    misses = 0
    for day_index in range(options.days):
        if codex_budget.exhausted() or misses >= MAX_CONSECUTIVE_MISSES:
            break
        for _ in range(options.codex_rollouts_per_day):
            if codex_budget.exhausted() or misses >= MAX_CONSECUTIVE_MISSES:
                break
            unit, pool_entry = build_codex_rollout(ctx, day_index, rollout_index)
            rollout_index += 1
            if commit_unit(
                codex_root, unit, codex_budget, options.zstd_fraction, ctx.rng, ctx.counters, zstd_selected
            ):
                ctx.codex_pool.append(pool_entry)
                misses = 0
            else:
                misses += 1

    if zstd_selected:
        compress_selected(zstd_selected)
        ctx.counters.codex_zstd_rollouts = len(zstd_selected)

    disk_bytes = 0
    files_written = 0
    for path in out_dir.rglob("*"):
        if path.is_file():
            disk_bytes += path.stat().st_size
            files_written += 1

    counters = ctx.counters
    return {
        "seed": options.seed,
        "out_dir": str(out_dir),
        "max_bytes": options.max_bytes,
        "content_bytes": claude_budget.used + codex_budget.used,
        "claude_content_bytes": claude_budget.used,
        "codex_content_bytes": codex_budget.used,
        "disk_bytes": disk_bytes,
        "files_written": files_written,
        "claude_sessions": counters.claude_sessions,
        "claude_resumed_sessions": counters.claude_resumed_sessions,
        "claude_subagent_files": counters.claude_subagent_files,
        "claude_workflow_subagent_files": counters.claude_workflow_subagent_files,
        "codex_rollouts": counters.codex_rollouts,
        "codex_forked_rollouts": counters.codex_forked_rollouts,
        "codex_token_usage_record_rollouts": counters.codex_token_usage_record_rollouts,
        "codex_zstd_rollouts": counters.codex_zstd_rollouts,
        "usage_records": counters.usage_records,
        "days": options.days,
    }


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out", type=Path, required=True, help="output directory (created if absent)")
    parser.add_argument("--seed", type=int, default=1, help="deterministic seed (default: 1)")
    parser.add_argument(
        "--max-bytes",
        type=int,
        default=256 * 1024 * 1024,
        help="total content byte cap across both agents, or 0 for unlimited "
        "(bounded only by --days and the per-day session/rollout counts); default 256 MiB",
    )
    parser.add_argument(
        "--claude-fraction",
        type=float,
        default=0.2,
        help="share of --max-bytes allotted to Claude content, 0..1 (default: 0.2)",
    )
    parser.add_argument(
        "--content-padding-bytes",
        type=int,
        default=0,
        help="extra bytes appended to non-usage content (prompts, replies, tool "
        "descriptions), for testing that memory does not scale with raw bytes",
    )
    parser.add_argument(
        "--zstd-fraction",
        type=float,
        default=0.0,
        help="fraction of Codex rollouts written as .jsonl.zst; requires a zstd binary "
        "on PATH (default: 0.0)",
    )
    parser.add_argument("--resume-fraction", type=float, default=0.15, help="fraction of Claude sessions that resume a prior session (default: 0.15)")
    parser.add_argument("--subagent-fraction", type=float, default=0.25, help="fraction of eligible turns that spawn a Claude subagent (default: 0.25)")
    parser.add_argument("--workflow-fraction", type=float, default=0.2, help="fraction of subagent spawns that nest a workflow subagent (default: 0.2)")
    parser.add_argument("--quota-fraction", type=float, default=0.05, help="fraction of Claude assistant blocks carrying quotaLimits (default: 0.05)")
    parser.add_argument("--fork-fraction", type=float, default=0.1, help="fraction of Codex rollouts that fork a prior rollout with copied history (default: 0.1)")
    parser.add_argument("--token-usage-record-fraction", type=float, default=0.5, help="fraction of Codex rollouts using token_usage_record instead of legacy cumulative counters (default: 0.5)")
    parser.add_argument("--claude-projects", type=int, default=5, help="number of distinct Claude project directories to rotate through (default: 5)")
    parser.add_argument(
        "--days",
        type=int,
        default=None,
        help=f"number of days spanned by generated sessions/rollouts "
        f"(default: {DEFAULT_DAYS}, or scaled up for a very large --max-bytes)",
    )
    parser.add_argument(
        "--claude-sessions-per-day",
        type=int,
        default=None,
        help="Claude sessions generated per day, capped by --max-bytes "
        "(default: scaled so --max-bytes, not this count, is normally the binding limit)",
    )
    parser.add_argument(
        "--codex-rollouts-per-day",
        type=int,
        default=None,
        help="Codex rollouts generated per day, capped by --max-bytes "
        "(default: scaled so --max-bytes, not this count, is normally the binding limit)",
    )
    args = parser.parse_args(argv)

    for name in (
        "claude_fraction",
        "resume_fraction",
        "subagent_fraction",
        "workflow_fraction",
        "quota_fraction",
        "fork_fraction",
        "token_usage_record_fraction",
        "zstd_fraction",
    ):
        value = getattr(args, name)
        if not 0.0 <= value <= 1.0:
            parser.error(f"--{name.replace('_', '-')} must be between 0 and 1, got {value}")
    if args.max_bytes < 0:
        parser.error("--max-bytes must be >= 0 (0 means unlimited)")
    if args.content_padding_bytes < 0:
        parser.error("--content-padding-bytes must be >= 0")
    if args.content_padding_bytes > MAX_RECORD_PADDING_BYTES:
        parser.error(
            f"--content-padding-bytes must be <= {MAX_RECORD_PADDING_BYTES} "
            "(the engine's per-record ceiling is 64 MiB; padding several fields per "
            "record must stay well under that)"
        )
    if args.days is not None and args.days <= 0:
        parser.error("--days must be positive")
    if (args.claude_sessions_per_day is not None and args.claude_sessions_per_day < 0) or (
        args.codex_rollouts_per_day is not None and args.codex_rollouts_per_day < 0
    ):
        parser.error("--claude-sessions-per-day and --codex-rollouts-per-day must be >= 0")
    if args.claude_projects <= 0:
        parser.error("--claude-projects must be positive")
    if args.zstd_fraction > 0 and shutil.which("zstd") is None:
        parser.error(
            "--zstd-fraction requires a zstd command-line binary on PATH; install one "
            "(e.g. `brew install zstd` or `apt-get install zstd`) or pass --zstd-fraction 0"
        )
    return args


def resolve_schedule(max_bytes: int | None, claude_fraction: float, days: int | None) -> tuple[int, int, int]:
    """Fills in unset --days/--*-per-day so --max-bytes is normally the binding limit.

    Without this, a fixed small default count (e.g. 40/day) becomes the actual limit
    once --max-bytes is large enough that it would need more scheduled units than that
    to fill up, silently producing a corpus far smaller than requested (see the
    docstring's `COUNT_HEADROOM` comment above).
    """
    resolved_days = days if days is not None else DEFAULT_DAYS
    if max_bytes is None:
        return resolved_days, 60, 60
    claude_budget = max_bytes * claude_fraction
    codex_budget = max_bytes - claude_budget
    claude_units = max(10, COUNT_HEADROOM * claude_budget / MIN_UNIT_BYTES_ESTIMATE)
    codex_units = max(10, COUNT_HEADROOM * codex_budget / MIN_UNIT_BYTES_ESTIMATE)
    claude_per_day = max(5, -(-int(claude_units) // resolved_days))
    codex_per_day = max(5, -(-int(codex_units) // resolved_days))
    return resolved_days, claude_per_day, codex_per_day


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    max_bytes = None if args.max_bytes == 0 else args.max_bytes
    default_days, default_claude_per_day, default_codex_per_day = resolve_schedule(
        max_bytes, args.claude_fraction, args.days
    )
    options = Options(
        seed=args.seed,
        max_bytes=max_bytes,
        claude_fraction=args.claude_fraction,
        content_padding_bytes=args.content_padding_bytes,
        zstd_fraction=args.zstd_fraction,
        resume_fraction=args.resume_fraction,
        subagent_fraction=args.subagent_fraction,
        workflow_fraction=args.workflow_fraction,
        quota_fraction=args.quota_fraction,
        fork_fraction=args.fork_fraction,
        token_usage_record_fraction=args.token_usage_record_fraction,
        claude_projects=args.claude_projects,
        days=default_days,
        claude_sessions_per_day=args.claude_sessions_per_day
        if args.claude_sessions_per_day is not None
        else default_claude_per_day,
        codex_rollouts_per_day=args.codex_rollouts_per_day
        if args.codex_rollouts_per_day is not None
        else default_codex_per_day,
    )
    out_dir = args.out
    out_dir.mkdir(parents=True, exist_ok=True)
    if any(out_dir.iterdir()):
        print(f"error: output directory is not empty: {out_dir}", file=sys.stderr)
        return 1
    summary = generate(options, out_dir)
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
