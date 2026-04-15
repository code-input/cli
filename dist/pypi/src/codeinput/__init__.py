"""CodeInput CLI - A powerful CLI tool for CODEOWNERS file management."""

try:
    from importlib.metadata import version as _version

    __version__ = _version("codeinput")
except Exception:
    __version__ = "0.0.0"
