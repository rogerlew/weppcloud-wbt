from __future__ import annotations

import os
import platform
from pathlib import Path


def whitebox_tools_exe() -> str:
    exe_name = "whitebox_tools.exe" if platform.system() == "Windows" else "whitebox_tools"

    env_path = os.environ.get("WHITEBOX_TOOLS_EXE")
    if env_path:
        env_candidate = Path(env_path).expanduser()
        if env_candidate.exists():
            return str(env_candidate)
        raise FileNotFoundError(
            f"WHITEBOX_TOOLS_EXE is set to '{env_path}', but that path does not exist."
        )

    packaged = Path(__file__).resolve().parent / "bin" / exe_name
    if packaged.exists():
        return str(packaged)

    raise FileNotFoundError(
        f"Could not find packaged {exe_name}. Set WHITEBOX_TOOLS_EXE "
        "or install a platform wheel for weppcloud-wbt."
    )
