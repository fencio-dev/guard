# Data Plane Restructuring Plan

## 1. Objective

The goal is to consolidate all data plane-related code into a single `data_plane` directory. This includes moving the `proto` and `semantic-sandbox` directories into `data_plane` and updating all references to these modules throughout the codebase.

## 2. Implementation Steps

### Phase 1: File System Restructuring

| # | Task Description | Status | Rationale |
|---|---|---|---|
| 1.1 | Move the `proto/` directory into `data_plane/`. | [ ] | Consolidate Protobuf definitions with the data plane code. |
| 1.2 | Move the `semantic-sandbox/` directory into `data_plane/`. | [ ] | Consolidate the semantic sandbox with the data plane code. |

### Phase 2: Code and Configuration Updates

| # | Task Description | Status | Rationale |
|---|---|---|---|
| 2.1 | Update `data_plane/tupl_dp/bridge/build.rs` to reflect the new path of the `proto` directory. | [ ] | Ensure the gRPC/Protobuf code can still be compiled. |
| 2.2 | Update `deployment/Dockerfile.security` to copy the `proto` and `semantic-sandbox` directories from their new location within `data_plane`. | [ ] | Fix the Docker build process for the security stack. |
| 2.3 | Update `.gitignore` to correctly ignore build artifacts in the new `data_plane/semantic-sandbox` path. | [ ] | Maintain a clean git history. |
| 2.4 | Update `management_plane/app/config.py` to point to the new location of the `libsemantic_sandbox.so` library. | [ ] | Ensure the management plane can still load the FFI library. |
| 2.5 | Update any remaining documentation or script files that reference the old paths. | [ ] | Maintain consistency across the project. |

## 3. Next Steps

Upon user approval of this plan, I will proceed with the execution of Phase 1.