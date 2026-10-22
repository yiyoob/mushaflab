"""Write a dated, private backup of the raw sources and the manifest (outside git).

    python -m pipeline.archive [DEST]    # default DEST: ~/mushaflab-archive

Sources can change or disappear upstream; a checksum alone cannot restore them.
The archive is only written after every source verifies, and is never overwritten.
"""

import datetime
import gzip
import hashlib
import io
import sys
import tarfile
from pathlib import Path

from pipeline.fetch import MANIFEST_PATH, RAW_ROOT, verify
from pipeline.manifest import Manifest, load_manifest

DEFAULT_DEST = Path.home() / "mushaflab-archive"


class ArchiveError(RuntimeError):
    pass


def _add(tar: tarfile.TarFile, name: str, data: bytes) -> None:
    # Fixed metadata so the same inputs always give the same archive bytes.
    info = tarfile.TarInfo(name)
    info.size = len(data)
    info.mode = 0o444
    info.mtime = 0
    tar.addfile(info, io.BytesIO(data))


def write_archive(
    manifest: Manifest, manifest_path: Path, raw_root: Path, dest: Path, today: str
) -> Path:
    problems = verify(manifest, raw_root)
    if problems:
        details = "; ".join(p.message for p in problems)
        raise ArchiveError(f"verify failed, not archiving: {details}")
    manifest_bytes = manifest_path.read_bytes()
    tag = hashlib.sha256(manifest_bytes).hexdigest()[:8]
    out = dest / f"mushaflab-sources-{today}-{tag}.tar.gz"
    if out.exists():
        raise ArchiveError(f"{out} already exists")
    buf = io.BytesIO()
    with (
        gzip.GzipFile(fileobj=buf, mode="wb", mtime=0) as gz,
        tarfile.open(fileobj=gz, mode="w", format=tarfile.PAX_FORMAT) as tar,
    ):
        _add(tar, "MANIFEST.json", manifest_bytes)
        for src in sorted(manifest.sources, key=lambda s: s.file):
            _add(tar, f"raw/{src.file}", (raw_root / src.file).read_bytes())
    dest.mkdir(parents=True, exist_ok=True)
    data = buf.getvalue()
    out.write_bytes(data)
    Path(f"{out}.sha256").write_text(
        f"{hashlib.sha256(data).hexdigest()}  {out.name}\n", encoding="utf-8"
    )
    return out


def main(argv: list[str]) -> int:
    dest = Path(argv[1]).expanduser() if len(argv) > 1 else DEFAULT_DEST
    today = datetime.datetime.now(datetime.UTC).strftime("%Y%m%d")
    try:
        out = write_archive(load_manifest(MANIFEST_PATH), MANIFEST_PATH, RAW_ROOT, dest, today)
    except ArchiveError as err:
        print(f"archive: {err}", file=sys.stderr)
        return 1
    print(f"archive: wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
