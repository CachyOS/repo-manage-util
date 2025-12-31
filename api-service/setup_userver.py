#! /usr/bin/env python3
"""// Copyright (C) 2022-2025 Vladislav Nepogodin
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

if len(argv) != 2:
    print('Wrong number of arguments! Please specify directory.')
    exit(1)

SRC_PATH = argv[1]

def main():
    if not os.path.exists(SRC_PATH + "/third_party"):
        pathlib.Path(SRC_PATH + "/third_party").mkdir(parents=True, exist_ok=True)

    if not os.path.exists(SRC_PATH + "/third_party/userver"):
        res = subprocess.run(["git", "clone", "https://github.com/userver-framework/userver.git", SRC_PATH + "/third_party/userver"])
        if res.returncode:
            print("Failed to clone repository")
            exit(1)

        os.chdir(SRC_PATH + "/third_party/userver")
        subprocess.run(["git", "checkout", "d05b85fae948a9016c414c732373935bdfd375d2"])
        subprocess.run(["patch", "-Np1", "-i", SRC_PATH + "/fix-postgresql.patch"])


main()
