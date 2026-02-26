#!/usr/bin/env python3
"""Suggested intents seeding helper.

In the current Guard UI, suggested intents are hardcoded in:
  ui/src/components/SuggestedIntentsPanel.jsx

So there is no DB seeding step required for suggested intents today.
This script exists to make that explicit for fresh setup workflows.
"""

from pathlib import Path


def main() -> None:
    project_root = Path(__file__).resolve().parents[1]
    source = project_root / "ui" / "src" / "components" / "SuggestedIntentsPanel.jsx"

    if source.exists():
        print("Suggested intents are hardcoded in the UI.")
        print(f"No seed step required: {source}")
    else:
        print("Suggested intents source file was not found.")
        print("If this changed to backend storage, add a real seed path here.")


if __name__ == "__main__":
    main()
