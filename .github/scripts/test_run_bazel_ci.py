#!/usr/bin/env python3

import json
import os
import shutil
import subprocess
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


class RunBazelCiTest(unittest.TestCase):
    def run_wrapper(
        self, *options: str, remote: bool = False, cross: bool = True
    ) -> list[str]:
        bash = "bash"
        if os.name == "nt":
            git = shutil.which("git")
            self.assertIsNotNone(git, "Git Bash is required to test the Bazel wrapper")
            for parent in Path(git).parents:
                candidate = parent / "bin" / "bash.exe"
                if candidate.is_file():
                    bash = str(candidate)
                    break
            else:
                self.fail("Could not locate Git Bash beside git.exe")
        with TemporaryDirectory() as temp_dir:
            directory = Path(temp_dir)
            script = directory / "run-bazel-ci.sh"
            shutil.copy2(Path(__file__).with_name(script.name), script)
            bazel = directory / "run_bazel_with_buildbuddy.py"
            bazel.write_text(
                "#!/usr/bin/env python3\nimport json, sys\nprint(json.dumps(sys.argv[1:]))\n",
                encoding="utf-8",
                newline="\n",
            )
            bazel.chmod(0o755)
            env = os.environ.copy()
            for name in (
                "BAZEL_OUTPUT_USER_ROOT",
                "BAZEL_REPOSITORY_CACHE",
                "BAZEL_REPO_CONTENTS_CACHE",
                "CODEX_BAZEL_EXECUTION_LOG_COMPACT_DIR",
            ):
                env.pop(name, None)
            env["RUNNER_OS"] = "Windows"
            env["CODEX_BAZEL_WINDOWS_PATH"] = "C:/tools"
            env["BUILDBUDDY_API_KEY"] = "test-key" if remote else ""
            result = subprocess.run(
                [
                    bash,
                    script.as_posix(),
                    *(["--windows-cross-compile"] if cross else []),
                    "--",
                    "test",
                    *options,
                    "--",
                    "//example:test",
                ],
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            return json.loads(result.stdout.splitlines()[-1])

    def test_keyless_default_target_uses_native_msvc_toolchains(self) -> None:
        args = self.run_wrapper()
        self.assertIn("--host_platform=//:local_windows_msvc", args)
        self.assertIn("--config=ci-windows-msvc", args)
        self.assertIn("--jobs=8", args)

    def test_authenticated_cross_build_keeps_remote_toolchains(self) -> None:
        args = self.run_wrapper(remote=True)
        self.assertIn("--host_platform=//:rbe", args)
        self.assertIn("--config=ci-windows-cross", args)
        self.assertNotIn("--config=ci-windows-msvc", args)

    def test_explicit_target_platform_keeps_its_toolchain_selection(self) -> None:
        for options in (
            ("--platforms=//:windows_x86_64_gnullvm",),
            ("--platforms", "//:windows_x86_64_gnullvm"),
        ):
            with self.subTest(options=options):
                args = self.run_wrapper(*options)
                self.assertNotIn("--config=ci-windows-msvc", args)

    def test_explicit_host_platform_keeps_its_toolchain_selection(self) -> None:
        args = self.run_wrapper("--host_platform=//:custom_host")
        self.assertIn("--host_platform=//:custom_host", args)
        self.assertNotIn("--config=ci-windows-msvc", args)

    def test_direct_windows_build_keeps_gnu_toolchains(self) -> None:
        args = self.run_wrapper(cross=False)
        self.assertNotIn("--config=ci-windows-msvc", args)
        self.assertNotIn("--host_platform=//:local_windows_msvc", args)


if __name__ == "__main__":
    unittest.main()
