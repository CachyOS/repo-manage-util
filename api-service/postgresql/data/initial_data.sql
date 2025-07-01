INSERT INTO repositories (id, repo_name, repo_desc, created_at, updated_at) VALUES
('c9a425b0-ea9e-45b0-ab9a-c8a7255dc63a', 'test1', NULL, '2025-06-30 15:14:59.470854+00', '2025-06-30 15:14:59.470854+00'),
('a1a1a1a1-a1a1-41a1-a1a1-a1a1a1a1a1a1', 'repo1', 'Core production packages', '2025-07-01 10:00:00.000000+00', '2025-07-01 10:00:00.000000+00'),
('b2b2b2b2-b2b2-42b2-b2b2-b2b2b2b2b2b2', 'repo2', 'Community-maintained packages and tools', '2025-07-01 11:00:00.000000+00', '2025-07-01 11:00:00.000000+00'),
('c3c3c3c3-c3c3-43c3-c3c3-c3c3c3c3c3c3', 'repo3', 'Testing and unstable packages', '2025-07-01 12:00:00.000000+00', '2025-07-01 12:00:00.000000+00');

INSERT INTO packages (
    id, repo_name, pkg_name, pkg_version, pkg_filename, pkg_base, pkg_desc,
    pkg_groups, pkg_url, pkg_license, pkg_arch, pkg_builddate, pkg_packager,
    pkg_csize, pkg_isize, pkg_sha256sum, pkg_pgpsig, pkg_replaces,
    pkg_depends, pkg_optdepends, pkg_makedepends, pkg_checkdepends,
    pkg_conflicts, pkg_provides, pkg_files, updated
) VALUES
(
    '316cddc5-1ec6-4a32-82d2-f12f55ed2b0b', 'test1', 'cachyos-cli-installer-new', '0.7.0-3', 'cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst', 'cachyos-cli-installer-new', 'New CLI installer for CachyOS',
    ARRAY[]::TEXT[], 'https://github.com/cachyos/new-cli-installer', ARRAY['GPL-3.0-or-later'], 'x86_64', '2024-07-11 08:23:01+00', 'Unknown Packager',
    977091, 2899347, 'c68448df79cb521edb9acf5b41656114049c79e6b3f897b91a8589e3b594815b', NULL, ARRAY[]::TEXT[],
    ARRAY['fzf', 'gawk', 'chwd', 'cachyos-rate-mirrors', 'curl'], ARRAY[]::TEXT[], ARRAY['cmake', 'ninja', 'git'], ARRAY[]::TEXT[],
    ARRAY[]::TEXT[], ARRAY[]::TEXT[], ARRAY[]::TEXT[], '2025-06-30 15:14:59.475235+00'
),
(
    '70671caf-4482-414a-a437-3a8b965022e4', 'repo1', 'dolt', '1.30.5-1.1', 'dolt-1.30.5-1.1-x86_64.pkg.tar.zst', 'dolt', 'Git for data! A version controlled relational database',
    ARRAY[]::TEXT[], 'https://www.dolthub.com', ARRAY['Apache'], 'x86_64', '2024-01-04 16:18:34+00', 'CachyOS <admin@cachyos.org>',
    18527187, 93412792, '36f26a9a520800f3a2c577f07334ca1c209bcd3ac6c457108abd417ec242d299', NULL, ARRAY[]::TEXT[],
    ARRAY['glibc'], ARRAY[]::TEXT[], ARRAY['go'], ARRAY[]::TEXT[],
    ARRAY[]::TEXT[], ARRAY[]::TEXT[], '{usr/,usr/bin/,usr/bin/dolt}'::TEXT[], '2025-07-01 10:05:11.123456+00'
),
(
    'c7b24f18-91a8-44b8-a705-51087e4f9c86', 'repo2', 'dwl-git', '0.2.1.r34.2d9740c-1', 'dwl-git-0.2.1.r34.2d9740c-1-x86_64.pkg.tar.zst', 'dwl-git', 'Simple, hackable dynamic tiling Wayland compositor (dwm for Wayland)',
    ARRAY[]::TEXT[], 'https://github.com/djpohly/dwl', ARRAY['GPL'], 'x86_64', '2021-12-19 19:56:01+00', 'Unknown Packager',
    36865, 60224, '972e56c771f063ab8ac33e55f3b74b40d0e9359601829392c6dcaca2e57cccfa', NULL, ARRAY[]::TEXT[],
    ARRAY['wlroots>=0.13'], ARRAY['xorg-xwayland: for XWayland support'], ARRAY['git', 'wayland-protocols'], ARRAY[]::TEXT[],
    ARRAY['dwl'], ARRAY['dwl'], ARRAY[]::TEXT[], '2025-07-01 11:05:22.54321+00'
),
(
    '55a2c892-c94e-4083-92b0-3027e27cff40', 'repo2', 'dwm', '6.2-4', 'dwm-6.2-4-x86_64.pkg.tar.zst', 'dwm', 'A dynamic window manager for X',
    ARRAY[]::TEXT[], 'https://dwm.suckless.org', ARRAY['MIT'], 'x86_64', '2022-01-13 21:59:32+00', 'Unknown Packager',
    46281, 60255, 'f57ad70fec79f7be6fd6d57a0245a31d983d3d6ca70dafd66cc1a490642482a5', NULL, ARRAY[]::TEXT[],
    ARRAY['libx11', 'libxinerama', 'libxft', 'freetype2', 'st', 'dmenu'], ARRAY[]::TEXT[], ARRAY[]::TEXT[], ARRAY[]::TEXT[],
    ARRAY[]::TEXT[], ARRAY[]::TEXT[], ARRAY[]::TEXT[], '2025-07-01 11:06:00.000000+00'
),
(
    '50787d29-595b-4817-bd70-2507bcbd6025', 'repo3', 'lightdm-webkit2-theme-arch', '1:0.1-1', 'lightdm-webkit2-theme-arch-1:0.1-1-any.pkg.tar.zst', 'lightdm-webkit2-theme-arch', 'Minimal theme for lightdm-webkit2-greeter using humorous wallpapers about Arch Linux.',
    ARRAY[]::TEXT[], 'https://gitlab.com/kenogo/lightdm-webkit2-theme-arch', ARRAY['WTFPL'], 'any', '2021-11-13 19:38:30+00', 'Unknown Packager',
    735288, 1195890, '8efceab06752f7d4264e7da4f020f09a9031e90cf8cb9c6f2538df8803e47ee9', NULL, ARRAY[]::TEXT[],
    ARRAY['lightdm-webkit2-greeter'], ARRAY[]::TEXT[], ARRAY['git'], ARRAY[]::TEXT[],
    ARRAY[]::TEXT[], ARRAY[]::TEXT[], ARRAY[]::TEXT[], '2025-07-01 12:05:00.000000+00'
),
(
    '6ec44943-4a4d-486a-a8d8-8da17282060a', 'test1', 'plymouth-theme-hud-3-git', 'r38.bf2f570-1', 'plymouth-theme-hud-3-git-r38.bf2f570-1-any.pkg.tar.zst', 'plymouth-themes-adi1090x-pack3-git', 'The plymouth theme collection by adi1090x',
    ARRAY[]::TEXT[], 'https://github.com/adi1090x/plymouth-themes', ARRAY['GPL'], 'any', '2021-10-13 19:23:03+00', 'Unknown Packager',
    3157448, 3372705, '8087d27aee81793552e8f1ae97d11a3e36f3da98135de95a7ec799ad29da2eab', NULL, ARRAY[]::TEXT[],
    ARRAY['plymouth'], ARRAY[]::TEXT[], ARRAY[]::TEXT[], ARRAY[]::TEXT[],
    ARRAY[]::TEXT[], ARRAY[]::TEXT[], ARRAY[]::TEXT[], '2025-06-30 15:14:59.48523+00'
),
(
    '3c6e9888-ba62-4376-9f66-289ad7db5f61', 'repo2', 'st', '0.8.4-2', 'st-0.8.4-2-x86_64.pkg.tar.zst', 'st', 'A simple virtual terminal emulator for X.',
    ARRAY[]::TEXT[], 'https://st.suckless.org', ARRAY['MIT'], 'x86_64', '2021-12-19 20:28:08+00', 'Unknown Packager',
    63374, 102639, 'a50095e352aa294a970b3f613cb9a84dfc4309e8d8a64a39d9f38d6ccbf937a6', NULL, ARRAY[]::TEXT[],
    ARRAY['libxft'], ARRAY[]::TEXT[], ARRAY[]::TEXT[], ARRAY[]::TEXT[],
    ARRAY[]::TEXT[], ARRAY[]::TEXT[], ARRAY[]::TEXT[], '2025-07-01 11:07:00.000000+00'
),
(
    'fababe03-5621-46ee-a7d2-fcc8a5e90e50', 'repo1', 'nginx', '1.24.0-1', 'nginx-1.24.0-1-x86_64.pkg.tar.zst', 'nginx', 'A lightweight HTTP and reverse proxy server',
    ARRAY['web-server'], 'https://nginx.org/', ARRAY['BSD-2-Clause'], 'x86_64', '2025-05-20 12:00:00+00', 'Repo Maintainer <maintainer@example.com>',
    750000, 2500000, 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', NULL, ARRAY[]::TEXT[],
    ARRAY['openssl', 'zlib', 'pcre'], ARRAY[]::TEXT[], ARRAY['make'], ARRAY[]::TEXT[],
    ARRAY[]::TEXT[], ARRAY['web-server'], ARRAY['/usr/bin/nginx'], '2025-07-01 10:10:00.000000+00'
),
(
    'cfcb8a55-2133-41f3-b969-393987ee6380', 'repo1', 'redis', '7.2.4-1', 'redis-7.2.4-1-x86_64.pkg.tar.zst', 'redis', 'An in-memory data structure store',
    ARRAY['database'], 'https://redis.io/', ARRAY['BSD-3-Clause'], 'x86_64', '2025-05-21 14:00:00+00', 'Repo Maintainer <maintainer@example.com>',
    2100000, 8500000, 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2', NULL, ARRAY[]::TEXT[],
    ARRAY['glibc'], ARRAY[]::TEXT[], ARRAY['make', 'gcc'], ARRAY[]::TEXT[],
    ARRAY[]::TEXT[], ARRAY['database-server'], ARRAY['/usr/bin/redis-server'], '2025-07-01 10:11:00.000000+00'
),
(
    '09fa1e42-90a7-4905-b8dd-8d509c3a7fc9', 'repo2', 'neovim', '0.9.5-1', 'neovim-0.9.5-1-x86_64.pkg.tar.zst', 'neovim', 'Vim-fork focused on extensibility and usability',
    ARRAY['editor'], 'https://neovim.io/', ARRAY['Apache-2.0'], 'x86_64', '2025-06-10 18:30:00+00', 'Community Packager <comm@example.com>',
    4500000, 25000000, 'f0e9d8c7b6a5f0e9d8c7b6a5f0e9d8c7b6a5f0e9d8c7b6a5f0e9d8c7b6a5f0e9', NULL, ARRAY['vim'],
    ARRAY['libuv', 'luajit'], ARRAY['python-pynvim: for python plugin support'], ARRAY['cmake', 'ninja'], ARRAY[]::TEXT[],
    ARRAY[]::TEXT[], ARRAY['vim-plugin-host'], ARRAY['/usr/bin/nvim'], '2025-07-01 11:15:00.000000+00'
),
(
    '44c40209-356e-4a25-b1b5-95d28233634f', 'repo2', 'ripgrep', '14.1.0-1', 'ripgrep-14.1.0-1-x86_64.pkg.tar.zst', 'ripgrep', 'A line-oriented search tool that recursively searches the current directory for a regex pattern',
    ARRAY['utility'], 'https://github.com/BurntSushi/ripgrep', ARRAY['MIT'], 'x86_64', '2025-06-15 09:00:00+00', 'Community Packager <comm@example.com>',
    2300000, 8000000, 'c7b6a5f0e9d8c7b6a5f0e9d8c7b6a5f0e9d8c7b6a5f0e9d8c7b6a5f0e9d8c7b6', NULL, ARRAY[]::TEXT[],
    ARRAY['glibc'], ARRAY['pcre2: for look-around and backreference support'], ARRAY['rust'], ARRAY[]::TEXT[],
    ARRAY[]::TEXT[], ARRAY[]::TEXT[], ARRAY['/usr/bin/rg'], '2025-07-01 11:16:00.000000+00'
),
(
    '95a974f5-e8b1-41e5-9201-b81f0e721112', 'repo3', 'docker', '26.1.1-1', 'docker-26.1.1-1-x86_64.pkg.tar.zst', 'docker', 'An open platform for developing, shipping, and running applications',
    ARRAY['containerization'], 'https://www.docker.com/', ARRAY['Apache-2.0'], 'x86_64', '2025-06-25 20:00:00+00', 'Unstable Builds <test@example.com>',
    45000000, 250000000, 'b6a5f0e9d8c7b6a5f0e9d8c7b6a5f0e9d8c7b6a5f0e9d8c7b6a5f0e9d8c7b6a5', NULL, ARRAY[]::TEXT[],
    ARRAY['containerd', 'runc'], ARRAY[]::TEXT[], ARRAY['go'], ARRAY[]::TEXT[],
    ARRAY[]::TEXT[], ARRAY[]::TEXT[], ARRAY['/usr/bin/docker'], '2025-07-01 12:20:00.000000+00'
),
(
    'ab7cc25d-e45f-42f6-a48d-300b5e065db7', 'repo3', 'python-requests', '2.31.0-1', 'python-requests-2.31.0-1-any.pkg.tar.zst', 'python-requests', 'Elegant and simple HTTP library for Python',
    ARRAY['python-libs'], 'https://requests.readthedocs.io/', ARRAY['Apache-2.0'], 'any', '2025-06-28 10:00:00+00', 'Unstable Builds <test@example.com>',
    150000, 800000, 'a5f0e9d8c7b6a5f0e9d8c7b6a5f0e9d8c7b6a5f0e9d8c7b6a5f0e9d8c7b6a5f0', NULL, ARRAY[]::TEXT[],
    ARRAY['python', 'python-urllib3', 'python-charset-normalizer'], ARRAY[]::TEXT[], ARRAY['python-setuptools'], ARRAY[]::TEXT[],
    ARRAY[]::TEXT[], ARRAY[]::TEXT[], ARRAY[]::TEXT[], '2025-07-01 12:21:00.000000+00'
);
