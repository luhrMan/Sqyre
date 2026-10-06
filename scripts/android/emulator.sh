#!/usr/bin/env bash
# Run Sqyre on an x86_64 Android emulator (KVM).
#
# Usage:
#   emulator.sh start [--headless]   boot the AVD, install bin/sqyre-debug.apk, launch it
#   emulator.sh install              reinstall the APK, enable the accessibility service
#   emulator.sh screenshot [FILE]    PNG of the screen (default: bin/emulator.png)
#   emulator.sh adb ARGS...          run adb against the emulator
#   emulator.sh stop
#
# Runs the SDK emulator directly when `emulator` and a writable /dev/kvm are present.
# Otherwise (host without the SDK, or the devcontainer, which has no /dev/kvm) it runs
# inside the devcontainer image via Docker with /dev/kvm and the X11 socket passed in.
#
# Env:
#   SQYRE_ANDROID_IMAGE   Docker image with the SDK + emulator (default: sqyre-dev-android;
#                         built from .devcontainer/Dockerfile when missing)
#   SQYRE_EMULATOR_GPU    emulator -gpu mode (default: swangle_indirect)
set -euo pipefail
_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$_here/../lib/repo-root.sh"

AVD=sqyre
SYSTEM_IMAGE="system-images;android-35;google_apis;x86_64"
PKG=com.sqyre.app
APK="$REPO_ROOT/bin/sqyre-debug.apk"
CONTAINER=sqyre-android-emulator
DOCKER_IMAGE="${SQYRE_ANDROID_IMAGE:-sqyre-dev-android}"
BOOT_TIMEOUT_S=300

die() {
	echo "emulator.sh: $*" >&2
	exit 1
}

can_run_local() {
	[ -n "${SQYRE_EMULATOR_IN_CONTAINER:-}" ] && return 0
	command -v emulator >/dev/null 2>&1 && [ -r /dev/kvm ] && [ -w /dev/kvm ]
}

container_running() {
	[ "$(docker inspect -f '{{.State.Running}}' "$CONTAINER" 2>/dev/null)" = true ]
}

# Re-run a subcommand inside the running emulator container.
in_container() {
	container_running || die "emulator is not running (make android-emulator)"
	docker exec "$CONTAINER" bash /workspace/scripts/android/emulator.sh "$@"
}

ensure_avd() {
	if ! avdmanager list avd -c 2>/dev/null | grep -qx "$AVD"; then
		echo "Creating AVD $AVD ($SYSTEM_IMAGE)"
		echo no | avdmanager create avd -n "$AVD" -k "$SYSTEM_IMAGE" -d pixel_6 --force
	fi
}

emulator_args() {
	local headless=$1
	local args=(-avd "$AVD" -no-snapshot-save -no-boot-anim -memory 4096 -cores 4
		-gpu "${SQYRE_EMULATOR_GPU:-swangle_indirect}")
	if [ "$headless" = 1 ]; then
		args+=(-no-window -no-audio)
	fi
	printf '%s\n' "${args[@]}"
}

wait_boot() {
	adb wait-for-device
	local waited=0
	until [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = 1 ]; do
		[ "$waited" -lt "$BOOT_TIMEOUT_S" ] || die "boot did not finish within ${BOOT_TIMEOUT_S}s"
		sleep 2
		waited=$((waited + 2))
	done
	echo "Emulator booted"
}

install_apk() {
	[ -f "$APK" ] || die "no $APK (make android ANDROID_ABIS=\"arm64-v8a x86_64\")"
	unzip -l "$APK" | grep 'lib/x86_64/libsqyre_app.so' >/dev/null \
		|| die "$APK has no x86_64 library (make android ANDROID_ABIS=\"arm64-v8a x86_64\")"
	local out
	if ! out="$(adb install -r -g "$APK" 2>&1)"; then
		case "$out" in
		*INSTALL_FAILED_UPDATE_INCOMPATIBLE*)
			# Debug builds from a fresh container get a fresh debug signing key.
			echo "Signature changed; reinstalling $PKG (app data is reset)"
			adb uninstall "$PKG" >/dev/null
			adb install -g "$APK"
			;;
		*) die "adb install failed: $out" ;;
		esac
	fi
	# Emulator-only shortcut for the Settings > Accessibility toggle.
	adb shell settings put secure enabled_accessibility_services "$PKG/$PKG.SqyreAccessibilityService"
	adb shell settings put secure accessibility_enabled 1
	adb shell am start -n "$PKG/.MainActivity" >/dev/null
	echo "Installed and launched $PKG"
}

start_local() {
	local headless=$1 foreground=$2
	ensure_avd
	if [ -n "${SQYRE_EMULATOR_IN_CONTAINER:-}" ] && [ -n "${ANDROID_AVD_HOME:-}" ]; then
		# A fresh container cannot share the AVD with a live emulator (start_container
		# refuses a second one), so locks left by a crashed run are stale.
		rm -f "$ANDROID_AVD_HOME/$AVD.avd/"*.lock
	fi
	local args
	mapfile -t args < <(emulator_args "$headless")
	if [ "$foreground" = 1 ]; then
		exec emulator "${args[@]}"
	fi
	mkdir -p "$REPO_ROOT/target"
	emulator "${args[@]}" >"$REPO_ROOT/target/emulator.log" 2>&1 &
	echo "Emulator log: $REPO_ROOT/target/emulator.log"
	wait_boot
	install_apk
}

ensure_image() {
	docker image inspect "$DOCKER_IMAGE" >/dev/null 2>&1 && return 0
	echo "Building $DOCKER_IMAGE from .devcontainer/Dockerfile"
	docker build -f "$REPO_ROOT/.devcontainer/Dockerfile" -t "$DOCKER_IMAGE" "$REPO_ROOT"
}

start_container() {
	local headless=$1
	command -v docker >/dev/null 2>&1 || die "need the Android SDK emulator + /dev/kvm, or Docker"
	container_running && die "emulator is already running (make android-emulator-stop)"
	ensure_image
	# Inside the devcontainer, Docker runs on the host: mount host paths.
	local host_repo="${LOCAL_WORKSPACE_FOLDER:-$REPO_ROOT}"
	local run=(docker run -d --rm --name "$CONTAINER" --network host
		--device /dev/kvm --security-opt label=disable
		-v "$host_repo:/workspace" -w /workspace
		-e SQYRE_EMULATOR_IN_CONTAINER=1
		-e "SQYRE_EMULATOR_GPU=${SQYRE_EMULATOR_GPU:-}"
		-e ANDROID_USER_HOME=/workspace/target/android-user-home
		-e ANDROID_AVD_HOME=/workspace/target/android-user-home/avd)
	if [ -e /dev/kvm ]; then
		run+=(--group-add "$(stat -c %g /dev/kvm)")
	fi
	local mode=()
	if [ "$headless" = 1 ]; then
		mode=(--headless)
	else
		run+=(-e "DISPLAY=${DISPLAY:-:0}" -v /tmp/.X11-unix:/tmp/.X11-unix)
		if [ -n "${XAUTHORITY:-}" ] && [ -f "$XAUTHORITY" ]; then
			run+=(-e XAUTHORITY=/tmp/.Xauthority -v "$XAUTHORITY:/tmp/.Xauthority:ro")
		fi
	fi
	"${run[@]}" "$DOCKER_IMAGE" bash /workspace/scripts/android/emulator.sh start --foreground "${mode[@]}" >/dev/null
	echo "Emulator container: $CONTAINER (docker logs -f $CONTAINER)"
	in_container wait
	in_container install
}

cmd="${1:-start}"
shift || true
case "$cmd" in
start)
	headless=0 foreground=0
	for arg in "$@"; do
		case "$arg" in
		--headless) headless=1 ;;
		--foreground) foreground=1 ;;
		*) die "unknown start option: $arg" ;;
		esac
	done
	if can_run_local; then
		start_local "$headless" "$foreground"
	else
		start_container "$headless"
	fi
	;;
wait)
	if can_run_local; then wait_boot; else in_container wait; fi
	;;
install)
	if can_run_local; then install_apk; else in_container install; fi
	;;
screenshot)
	out="${1:-$REPO_ROOT/bin/emulator.png}"
	if can_run_local; then
		mkdir -p "$(dirname "$out")"
		adb exec-out screencap -p >"$out"
		echo "$out"
	else
		container_running || die "emulator is not running (make android-emulator)"
		docker exec "$CONTAINER" adb exec-out screencap -p >"$out"
		echo "$out"
	fi
	;;
adb)
	if can_run_local; then adb "$@"; else
		container_running || die "emulator is not running (make android-emulator)"
		docker exec "$CONTAINER" adb "$@"
	fi
	;;
stop)
	if container_running; then
		docker rm -f "$CONTAINER" >/dev/null
	elif command -v adb >/dev/null 2>&1; then
		adb emu kill || true
	fi
	echo "Emulator stopped"
	;;
*)
	die "unknown command: $cmd (start | install | screenshot | adb | stop)"
	;;
esac
