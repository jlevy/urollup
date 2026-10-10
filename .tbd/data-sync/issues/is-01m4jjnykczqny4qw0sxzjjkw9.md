---
type: is
id: is-01m4jjnykczqny4qw0sxzjjkw9
title: Honor Windows job-object memory limits in effective memory
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - memory
dependencies: []
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
created_at: 2026-10-10T09:35:36.027Z
updated_at: 2026-10-10T09:35:36.027Z
---
Follow-up to process-wide admission (uro-6pi8), filed from PR #27 review A finding A16 (https://github.com/jlevy/urollup/pull/27#pullrequestreview-5477063239). In 0.1, effective memory M on Windows is physical RAM (GlobalMemoryStatusEx). A process running inside a Windows job object with JOB_OBJECT_LIMIT_PROCESS_MEMORY or JOB_OBJECT_LIMIT_JOB_MEMORY (for example a CI runner or container) can be terminated or fail allocations well below 25% of physical RAM. Read the job limits with QueryInformationJobObject(JobObjectExtendedLimitInformation) for the current process's job, including nested jobs, and include ProcessMemoryLimit and JobMemoryLimit as allowances in M, with a source-naming label, as the Linux cgroup and rlimit allowances do (plan section 'Budget Source and Flag Semantics'). Keep the read-only query in the scoped unsafe form used for GlobalMemoryStatusEx; unreadable or absent limits contribute nothing. Test with a fixture shim on every platform and, on the Windows CI runner, by running the binary inside a job object with a memory limit. Not a 0.1 blocker.
