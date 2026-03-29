import logging
import tempfile
from pathlib import Path
import re
import os
from typing import Union

import pytest

import sysand


def test_basic_init(caplog: pytest.LogCaptureFixture) -> None:
    level = logging.DEBUG
    logging.basicConfig(level=level)
    caplog.set_level(level)

    with tempfile.TemporaryDirectory() as tmpdirname:
        sysand.init("test_basic_init", "a", "1.2.3", tmpdirname)

        assert caplog.record_tuples == [
            (
                "sysand_core.commands.init",
                logging.INFO,
                "    Creating interchange project `test_basic_init`",
            )
        ]
        with open(Path(tmpdirname) / ".project.json", "r") as f:
            assert (
                f.read()
                == '{\n  "name": "test_basic_init",\n  "publisher": "a",\n  "version": "1.2.3",\n  "usage": []\n}\n'
            )
        with open(Path(tmpdirname) / ".meta.json", "r") as f:
            assert re.match(
                r'\{\n  "index": \{\},\n  "created": "\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.(\d{6}|\d{9})Z"\n}\n',
                f.read(),
            )


def test_basic_env() -> None:
    with tempfile.TemporaryDirectory() as tmpdirname:
        env_path = Path(tmpdirname) / sysand.env.DEFAULT_ENV_NAME
        sysand.env.env(env_path)
        assert env_path.is_dir()
        assert (env_path / "entries.txt").is_file()
        assert os.stat(env_path / "entries.txt").st_size == 0


def test_end_to_end_install() -> None:
    """Test init, include, env create, install, add, exclude flow."""
    with tempfile.TemporaryDirectory() as tmp_main:
        with tempfile.TemporaryDirectory() as tmp_dep:
            tmp_main = Path(tmp_main).resolve()
            tmp_dep = Path(tmp_dep).resolve()
            sysand.init("test_end_to_end_install", "a", "1.2.3", tmp_main)
            sysand.init("test_end_to_end_install_dep", "a", "1.2.3", tmp_dep)

            with open(Path(tmp_main) / "src.sysml", "w") as f:
                f.write("package Src;")

            sysand.include(tmp_main, "src.sysml")

            with open(Path(tmp_dep) / "src_dep.sysml", "w") as f:
                f.write("package SrcDep;")

            sysand.include(tmp_dep, "src_dep.sysml")

            env_path = Path(tmp_main) / sysand.env.DEFAULT_ENV_NAME

            sysand.env.env(env_path)

            sysand.env.install_path(
                env_path, "urn:kpar:test_end_to_end_install_dep", tmp_dep
            )

            sysand.add(
                tmp_main, "urn:kpar:test_end_to_end_install_dep", "1.2.3"
            )

            # Verify .project.json has the usage
            import json

            with open(tmp_main / ".project.json") as f:
                proj = json.load(f)
            assert len(proj["usage"]) == 1
            assert proj["usage"][0]["resource"] == "urn:kpar:test_end_to_end_install_dep"

            sysand.exclude(tmp_main, "src.sysml")

            # Verify .meta.json no longer has the source
            with open(tmp_main / ".meta.json") as f:
                meta = json.load(f)
            assert "src.sysml" not in meta.get("index", {})


@pytest.mark.parametrize(
    "compression",
    [None, sysand.CompressionMethod.STORED, sysand.CompressionMethod.DEFLATED],
)
def test_build(compression: Union[sysand.CompressionMethod, None]) -> None:
    with tempfile.TemporaryDirectory() as tmp_main:
        tmp_main = Path(tmp_main).resolve()
        sysand.init("test_build", "a", "1.2.3", tmp_main)

        with open(tmp_main / "src.sysml", "w") as f:
            f.write("package Src;")

        sysand.include(tmp_main, "src.sysml")

        sysand.build(
            output_path=tmp_main / "test_build.kpar",
            project_path=tmp_main,
            compression=compression,
        )
