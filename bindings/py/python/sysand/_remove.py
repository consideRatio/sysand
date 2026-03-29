# SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
#
# SPDX-License-Identifier: MIT OR Apache-2.0

from __future__ import annotations

import sysand._sysand_core as sysand_rs  # type: ignore

from pathlib import Path


def remove(path: Path | str, iri: str) -> None:
    sysand_rs.usage_remove(str(path), iri)


__all__ = ["remove"]
