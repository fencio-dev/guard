"""Shared vocabulary helper for Management Plane modules."""
from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SDK_PATH = REPO_ROOT / "sdk" / "python"
if str(SDK_PATH) not in sys.path:
    sys.path.insert(0, str(SDK_PATH))

from tupl.vocabulary import VocabularyRegistry

VOCABULARY = VocabularyRegistry()
