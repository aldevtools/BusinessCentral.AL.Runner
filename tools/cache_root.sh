# Sourced by tools/machine_cleanup.sh and tools/disk_floor.sh: sets $cache to the cache root, in the order of
# crates/al-deps/src/cache.rs:32-52 (a value of only whitespace counts as unset): ALRUNNER_CACHE, AL_RUNTIME_CACHE,
# $AL_RUNNER_CACHE_ROOT/v3, $XDG_CACHE_HOME/al-runtime, else $HOME (or $USERPROFILE)/.cache/al-runtime.
# Exits 2 when the root is empty, "/" or $HOME itself: the scripts delete below it, so it must be a folder of its own.
_cache_var() { local v="${!1:-}"; [ -n "${v//[[:space:]]/}" ] && printf '%s' "$v"; }
cache="$(_cache_var ALRUNNER_CACHE)"
[ -n "$cache" ] || cache="$(_cache_var AL_RUNTIME_CACHE)"
if [ -z "$cache" ] && [ -n "$(_cache_var AL_RUNNER_CACHE_ROOT)" ]; then cache="$(_cache_var AL_RUNNER_CACHE_ROOT)/v3"; fi
if [ -z "$cache" ]; then
  _base="$(_cache_var XDG_CACHE_HOME)"
  if [ -z "$_base" ]; then
    _home="$(_cache_var HOME)"; [ -n "$_home" ] || _home="$(_cache_var USERPROFILE)"
    [ -n "$_home" ] && _base="$_home/.cache"
  fi
  [ -n "$_base" ] && cache="$_base/al-runtime"
fi
_norm="${cache%/}"
if [ -z "$_norm" ] || [ "$_norm" = "${HOME%/}" ]; then
  echo "cache root '$cache' is empty, / or the home folder: refusing (set ALRUNNER_CACHE to a folder of its own)" >&2
  exit 2
fi
