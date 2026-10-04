"""Generate deterministic CSV inputs for the Eck streaming benchmarks."""

import hashlib
import os
import tempfile
from pathlib import Path


DATA_DIRECTORY = Path(__file__).resolve().parent / "data"
EXPECTED_HASHES = {
    "large_integer.csv": "f73cf5737e1541c99d96281fe8077d788ca9cbd6b9f47e44c106493ae9533bae",
    "large_mixed.csv": "b67885eba203614ec557749a0dedaea884cfee615c33fce0348731a17e2ffdd2",
    "small_mixed.csv": "ccae9778e85d18eefb5d40648b2a3be36c04691f0bd6774a309efa12839f4958",
}


def file_hash(path: Path) -> str:
    """Return the SHA-256 digest of a file's bytes."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_dataset(filename: str, path: Path) -> None:
    """Write one benchmark dataset with its deterministic byte representation."""
    with path.open("w", encoding="utf-8", newline="\n") as output:
        if filename == "large_integer.csv":
            output.write("id,account,quantity\n")
            for index in range(100_000):
                output.write(f"{index},{100_000 + index},{index % 1_000}\n")
        elif filename == "large_mixed.csv":
            output.write("id,name,region,active,score\n")
            for index in range(100_000):
                score = index % 1_000
                active = "true" if index % 2 == 0 else "false"
                output.write(
                    f"{index},customer_{index},region_{index % 32},{active},"
                    f"{score // 10}.{score % 10}\n"
                )
        else:
            output.write("id,name,region,active\n")
            for index in range(256):
                active = "true" if index % 2 == 0 else "false"
                output.write(f"{index},user_{index},region_{index % 8},{active}\n")


def generate() -> None:
    """Keep valid datasets untouched and atomically repair missing or altered ones."""
    DATA_DIRECTORY.mkdir(parents=True, exist_ok=True)

    for filename, expected_hash in EXPECTED_HASHES.items():
        destination = DATA_DIRECTORY / filename
        if destination.is_file() and file_hash(destination) == expected_hash:
            print(f"OK: {filename} is current")
            continue

        status = "created" if not destination.exists() else "repaired"
        temporary_path = None
        try:
            with tempfile.NamedTemporaryFile(
                mode="w", encoding="utf-8", newline="\n", dir=DATA_DIRECTORY,
                prefix=f".{filename}.", suffix=".tmp", delete=False,
            ) as temporary_file:
                temporary_path = Path(temporary_file.name)
            write_dataset(filename, temporary_path)
            if file_hash(temporary_path) != expected_hash:
                raise RuntimeError(f"Generated data failed SHA-256 validation: {filename}")
            os.replace(temporary_path, destination)
            temporary_path = None
            print(f"{status}: {filename}")
        finally:
            if temporary_path is not None:
                temporary_path.unlink(missing_ok=True)


if __name__ == "__main__":
    generate()
