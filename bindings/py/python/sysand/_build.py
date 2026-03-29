from __future__ import annotations

from sysand._model import CompressionMethod
import sysand._sysand_core as sysand_rs  # type: ignore

from pathlib import Path


def build(
    output_path: str | Path,
    project_path: str | Path | None = None,
    compression: CompressionMethod | None = None,
) -> None:
    if project_path is not None:
        project_path = str(project_path)

    # comp = None if compression is None else _convert_compression(compression)
    comp = None if compression is None else compression.name
    sysand_rs.build(project_path, str(output_path), compression=comp)


__all__ = [
    "build",
]
