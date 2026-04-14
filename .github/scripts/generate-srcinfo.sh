#!/usr/bin/env bash
# Generate .SRCINFO from a PKGBUILD file.
# Usage: generate-srcinfo.sh <PKGBUILD_PATH>
#
# Sources the PKGBUILD in a restricted subshell and outputs .SRCINFO format.

set -uo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <PKGBUILD_PATH>" >&2
    exit 1
fi

pkgbuild="$1"

if [ ! -f "$pkgbuild" ]; then
    echo "Error: $pkgbuild not found" >&2
    exit 1
fi

(
    # Override functions that might cause side effects
    build() { :; }
    package() { :; }
    prepare() { :; }
    check() { :; }

    source "$pkgbuild"

    # Determine pkgbase (defaults to first pkgname)
    base="${pkgbase:-${pkgname[0]:-${pkgname:-}}}"

    # pkgbase section
    echo "pkgbase = ${base}"
    [ -n "${pkgdesc:-}" ] && printf '\tpkgdesc = %s\n' "$pkgdesc"
    [ -n "${pkgver:-}" ]  && printf '\tpkgver = %s\n' "$pkgver"
    [ -n "${pkgrel:-}" ]  && printf '\tpkgrel = %s\n' "$pkgrel"
    [ -n "${url:-}" ]     && printf '\turl = %s\n' "$url"

    # Array fields
    for v in "${arch[@]}";            do printf '\tarch = %s\n' "$v";           done 2>/dev/null || true
    for v in "${license[@]}";         do printf '\tlicense = %s\n' "$v";        done 2>/dev/null || true
    for v in "${depends[@]}";         do printf '\tdepends = %s\n' "$v";        done 2>/dev/null || true
    for v in "${makedepends[@]}";     do printf '\tmakedepends = %s\n' "$v";    done 2>/dev/null || true
    for v in "${checkdepends[@]}";    do printf '\tcheckdepends = %s\n' "$v";   done 2>/dev/null || true
    for v in "${optdepends[@]}";      do printf '\toptdepends = %s\n' "$v";     done 2>/dev/null || true
    for v in "${provides[@]}";        do printf '\tprovides = %s\n' "$v";       done 2>/dev/null || true
    for v in "${conflicts[@]}";       do printf '\tconflicts = %s\n' "$v";      done 2>/dev/null || true
    for v in "${replaces[@]}";        do printf '\treplaces = %s\n' "$v";       done 2>/dev/null || true
    for v in "${groups[@]}";          do printf '\tgroups = %s\n' "$v";         done 2>/dev/null || true
    for v in "${options[@]}";         do printf '\toptions = %s\n' "$v";        done 2>/dev/null || true
    for v in "${source[@]}";          do printf '\tsource = %s\n' "$v";         done 2>/dev/null || true
    for v in "${sha256sums[@]}";      do printf '\tsha256sums = %s\n' "$v";     done 2>/dev/null || true
    for v in "${sha512sums[@]}";      do printf '\tsha512sums = %s\n' "$v";     done 2>/dev/null || true
    for v in "${md5sums[@]}";         do printf '\tmd5sums = %s\n' "$v";        done 2>/dev/null || true

    # Architecture-specific arrays
    for v in "${source_x86_64[@]}";       do printf '\tsource_x86_64 = %s\n' "$v";       done 2>/dev/null || true
    for v in "${source_aarch64[@]}";      do printf '\tsource_aarch64 = %s\n' "$v";      done 2>/dev/null || true
    for v in "${sha256sums_x86_64[@]}";   do printf '\tsha256sums_x86_64 = %s\n' "$v";   done 2>/dev/null || true
    for v in "${sha256sums_aarch64[@]}";  do printf '\tsha256sums_aarch64 = %s\n' "$v";  done 2>/dev/null || true
    for v in "${sha512sums_x86_64[@]}";   do printf '\tsha512sums_x86_64 = %s\n' "$v";   done 2>/dev/null || true
    for v in "${sha512sums_aarch64[@]}";  do printf '\tsha512sums_aarch64 = %s\n' "$v";  done 2>/dev/null || true

    echo ""

    # pkgname sections
    if declare -p pkgname 2>/dev/null | grep -q 'declare -a'; then
        for name in "${pkgname[@]}"; do
            echo "pkgname = ${name}"
        done
    else
        echo "pkgname = ${pkgname}"
    fi
) 2>/dev/null
