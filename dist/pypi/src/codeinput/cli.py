"""CodeInput CLI wrapper that downloads and runs the native binary."""

import platform
import stat
import subprocess
import sys
from importlib.metadata import version as get_version
from pathlib import Path

BINARY_DIR = Path.home() / ".cache" / "codeinput"
GITHUB_REPO = "code-input/cli"
VERSION = get_version("codeinput")


def _get_platform_info():
    """Detect the current platform and return (os, arch) tuple."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    if system == "linux":
        os_name = "linux"
    elif system == "darwin":
        os_name = "macos"
    elif system == "windows":
        os_name = "windows"
    else:
        raise RuntimeError(f"Unsupported operating system: {system}")

    if machine in ("x86_64", "amd64"):
        arch = "x86_64"
    elif machine in ("aarch64", "arm64"):
        arch = "aarch64"
    else:
        raise RuntimeError(f"Unsupported architecture: {machine}")

    return os_name, arch


def _get_binary_name(os_name, arch):
    """Return the binary filename for the given platform."""
    if os_name == "windows":
        return f"ci-{os_name}-{arch}.exe"
    return f"ci-{os_name}-{arch}"


def _get_binary_path():
    """Return the local path where the binary should be stored."""
    os_name, arch = _get_platform_info()
    binary_name = _get_binary_name(os_name, arch)
    return BINARY_DIR / VERSION / binary_name


def _download_binary():
    """Download the binary from GitHub releases."""
    os_name, arch = _get_platform_info()
    binary_name = _get_binary_name(os_name, arch)
    url = f"https://github.com/{GITHUB_REPO}/releases/download/v{VERSION}/{binary_name}"

    binary_path = _get_binary_path()
    binary_path.parent.mkdir(parents=True, exist_ok=True)

    import urllib.request
    import urllib.error

    try:
        urllib.request.urlretrieve(url, binary_path)
    except urllib.error.HTTPError as e:
        print(f"Error downloading binary: {e}", file=sys.stderr)
        print(f"URL: {url}", file=sys.stderr)
        sys.exit(1)

    if os_name != "windows":
        binary_path.chmod(binary_path.stat().st_mode | stat.S_IEXEC)

    return binary_path


def _ensure_binary():
    """Ensure the binary is available, downloading if necessary."""
    binary_path = _get_binary_path()
    if binary_path.exists():
        return binary_path
    return _download_binary()


def main():
    """Run the CodeInput CLI."""
    try:
        binary_path = _ensure_binary()
    except Exception as e:
        print(f"Error setting up CodeInput binary: {e}", file=sys.stderr)
        sys.exit(1)

    result = subprocess.run([str(binary_path)] + sys.argv[1:])
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
