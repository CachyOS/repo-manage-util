#! /usr/bin/env python3
"""// Copyright (C) 2022-2026 Vladislav Nepogodin
//
// This file is part of CachyOS.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along
// with this program; if not, write to the Free Software Foundation, Inc.,
// 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
"""

from sys import argv
import os
import subprocess
import pathlib
import multiprocessing

if len(argv) != 2:
    print('Wrong number of arguments! Please specify directory.')
    exit(1)

SRC_PATH = argv[1]

def main():
    if not os.path.exists(SRC_PATH + "/third_party"):
        pathlib.Path(SRC_PATH + "/third_party").mkdir(parents=True, exist_ok=True)

    if not os.path.exists(SRC_PATH + "/third_party/postgresql"):
        res = subprocess.run(["git", "clone", "--depth", "1", "--branch", "REL_18_3", "https://github.com/postgres/postgres.git", SRC_PATH + "/third_party/postgresql"])
        if res.returncode:
            print("Failed to clone repository")
            exit(1)

        meson_options = [
            "--prefix=/usr",
            "--mandir=/usr/share/man",
            "--datadir=/usr/share/postgresql",
            "--sysconfdir=/etc",
            "--default-library=static",
            "--buildtype=release",
            "-Dssl=openssl",
            "-Ddocs=disabled",
            "-Dgssapi=disabled",
            "-Dicu=disabled",
            "-Dlibxml=disabled",
            "-Dlibxslt=disabled",
            "-Dplperl=disabled",
            "-Dplpython=disabled",
            "-Dreadline=disabled",
            "-Dpltcl=disabled",
            "-Dpam=disabled",
            "-Dldap=disabled",
            "-Dlibcurl=disabled",
            "-Dsystem_tzdata=/usr/share/zoneinfo",
            "-Duuid=e2fs",
            "-Dlz4=enabled",
            "-Dzstd=enabled",
            "-Drpath=false",
        ]

        os.chdir(SRC_PATH + "/third_party/postgresql")
        subprocess.run(["patch", "-Np1", "-i", SRC_PATH + "/cmake_postgresql.patch"])
        if subprocess.run(["meson", "setup", "build"] + meson_options).returncode:
            print("Failed to configure")
            exit(1)
        if subprocess.run(["meson", "compile", "-C", "build", "--jobs", str(multiprocessing.cpu_count())]).returncode:
            print("Failed to build")
            exit(1)
        if subprocess.run(["./create_combined_static.sh"]).returncode:
            print("Failed to created combined libpq archive")
            exit(1)

main()
