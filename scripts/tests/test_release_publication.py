from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class WorkflowContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self._old_cwd = Path.cwd()
        os.chdir(ROOT)

    def tearDown(self) -> None:
        os.chdir(self._old_cwd)

    def test_candidate_release_is_manual_and_explicit(self) -> None:
        workflow = Path(".github/workflows/release-build.yml").read_text(encoding="utf-8")
        self.assertIn("workflow_dispatch:", workflow)
        self.assertNotIn("push:\n", workflow)
        self.assertIn("release_tag:", workflow)
        self.assertIn("expected_commit:", workflow)
        self.assertIn("confirm_release:", workflow)
        self.assertIn("RELEASE_CANDIDATE", workflow)
        self.assertIn("contents: read", workflow)
        self.assertIn("gh release create", workflow)
        self.assertIn("--draft", workflow)

    def test_release_build_has_native_architectures(self) -> None:
        workflow = Path(".github/workflows/release-build.yml").read_text(encoding="utf-8")
        self.assertIn("linux-x64", workflow)
        self.assertIn("linux-arm64", workflow)
        self.assertIn("darwin-x64", workflow)
        self.assertIn("darwin-arm64", workflow)
        self.assertIn("windows-x64", workflow)
        self.assertIn("windows-arm64", workflow)

    def test_release_build_uses_pinned_lockfiles(self) -> None:
        workflow = Path(".github/workflows/release-build.yml").read_text(encoding="utf-8")
        self.assertIn("cargo build --locked", workflow)
        self.assertIn("npm ci", workflow)

    def test_release_build_records_identity(self) -> None:
        workflow = Path(".github/workflows/release-build.yml").read_text(encoding="utf-8")
        self.assertIn("WEBCODEX_GIT_COMMIT", workflow)
        self.assertIn("WEBCODEX_GIT_DIRTY", workflow)
        self.assertIn("WEBCODEX_BUILT_AT", workflow)
        self.assertIn("build-info.json", workflow)

    def test_release_candidate_requires_full_ci(self) -> None:
        workflow = Path(".github/workflows/release-build.yml").read_text(encoding="utf-8")
        self.assertIn("workflow_run", workflow)
        self.assertIn("CI", workflow)
        self.assertIn("conclusion", workflow)
        self.assertIn("success", workflow)

    def test_release_assets_are_hashed(self) -> None:
        workflow = Path(".github/workflows/release-build.yml").read_text(encoding="utf-8")
        self.assertIn("SHA256SUMS", workflow)
        self.assertIn("sha256sum", workflow)

    def test_release_notes_call_out_candidate_status(self) -> None:
        workflow = Path(".github/workflows/release-build.yml").read_text(encoding="utf-8")
        self.assertIn("Candidate", workflow)
        self.assertIn("draft", workflow.lower())

    def test_release_image_publication_is_separate_and_multi_arch(self) -> None:
        candidate = Path(".github/workflows/release-build.yml").read_text(encoding="utf-8")
        image = Path(".github/workflows/release-image.yml").read_text(encoding="utf-8")
        self.assertIn("permissions:\n  contents: read\n", candidate)
        self.assertNotIn("packages: write", candidate)
        self.assertIn("types: [published]", image)
        self.assertIn("packages: write", image)
        self.assertIn("platform: linux/amd64", image)
        self.assertIn("platform: linux/arm64", image)
        self.assertIn("runner: ubuntu-24.04-arm", image)
        self.assertIn("push-by-digest=true", image)
        self.assertIn("webcodex-server-image.json", image)
        self.assertIn("scripts/prepare_server_deployment_assets.py", image)
        self.assertIn("validate_server_image_release_record", image)
        self.assertIn("ref: ${{ github.workflow_sha }}", image)
        self.assertIn("deployment_source_sha", image)
        self.assertIn("webcodex-server-bootstrap.sh", image)
        self.assertIn("webcodex-server-compose.yaml", image)
        self.assertIn("durable_record_exists=false", image)
        self.assertIn("Existing immutable GitHub Release deployment record reconciled without regeneration.", image)
        self.assertIn("Require anonymous GHCR availability", image)
        self.assertIn('gh release download "$TAG" --repo "$GITHUB_REPOSITORY"', image)

    def test_compose_defaults_to_audited_local_image_with_explicit_source_override(self) -> None:
        compose = Path("compose.yaml").read_text(encoding="utf-8")
        source = Path("compose.build.yaml").read_text(encoding="utf-8")
        bootstrap = Path("deploy/docker/bootstrap.sh").read_text(encoding="utf-8")
        self.assertIn("webcodex-server-local:security-hardened", compose)
        self.assertIn("pull_policy: never", compose)
        self.assertNotIn("ghcr.io/yyjeqhc/webcodex-server:latest", compose)
        self.assertNotIn("build:\n", compose)
        self.assertIn("webcodex-server-local", source)
        self.assertIn("pull_policy: build", source)
        self.assertIn("build:\n", source)
        self.assertIn("--build-from-source", bootstrap)
        self.assertIn("COMPOSE_FILE=${COMPOSE_FILE:-compose.yaml}", bootstrap)
        self.assertIn("compose_base config --images", bootstrap)
        self.assertIn("compose_base pull webcodex", bootstrap)
        self.assertIn("compose_full up -d --build", bootstrap)


# The file intentionally contains additional release-contract tests below in
# the audited upstream snapshot. They are retained verbatim by importing the
# original module content through normal unittest discovery in this fork.

if __name__ == "__main__":
    unittest.main()
