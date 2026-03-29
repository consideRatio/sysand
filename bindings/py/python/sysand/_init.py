# SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
#
# SPDX-License-Identifier: MIT OR Apache-2.0

from __future__ import annotations

import sysand._sysand_core as sysand_rs  # type: ignore

from pathlib import Path


def init(
    name: str,
    publisher: str,
    version: str,
    path: str | Path = ".",
    license: str | None = None,
) -> None:
    if not Path(path).exists():
        Path(path).mkdir()

    sysand_rs.init(str(path), name=name, publisher=publisher, version=version, license=license)


__all__ = ["init"]
