#!/bin/bash
# Fresher configuration for fresher
# Generated: 2026-01-17T16:58:01Z
# Project type: generic

#──────────────────────────────────────────────────────────────────
# Mode Configuration
#──────────────────────────────────────────────────────────────────
export FRESHER_MODE="${FRESHER_MODE:-planning}"

#──────────────────────────────────────────────────────────────────
# Termination Settings
#──────────────────────────────────────────────────────────────────
export FRESHER_MAX_ITERATIONS="${FRESHER_MAX_ITERATIONS:-0}"
export FRESHER_SMART_TERMINATION="${FRESHER_SMART_TERMINATION:-true}"

#──────────────────────────────────────────────────────────────────
# Claude Code Settings
#──────────────────────────────────────────────────────────────────
export FRESHER_DANGEROUS_PERMISSIONS="${FRESHER_DANGEROUS_PERMISSIONS:-true}"
export FRESHER_MAX_TURNS="${FRESHER_MAX_TURNS:-50}"
export FRESHER_MODEL="${FRESHER_MODEL:-sonnet}"

#──────────────────────────────────────────────────────────────────
# Project Commands (detected: generic)
#──────────────────────────────────────────────────────────────────
export FRESHER_TEST_CMD="${FRESHER_TEST_CMD:-}"
export FRESHER_BUILD_CMD="${FRESHER_BUILD_CMD:-}"
export FRESHER_LINT_CMD="${FRESHER_LINT_CMD:-}"

#──────────────────────────────────────────────────────────────────
# Paths
#──────────────────────────────────────────────────────────────────
export FRESHER_LOG_DIR="${FRESHER_LOG_DIR:-.fresher/logs}"
export FRESHER_SPEC_DIR="${FRESHER_SPEC_DIR:-specs}"
export FRESHER_SRC_DIR="${FRESHER_SRC_DIR:-src}"

#──────────────────────────────────────────────────────────────────
# Docker (optional)
#──────────────────────────────────────────────────────────────────
export FRESHER_USE_DOCKER="${FRESHER_USE_DOCKER:-false}"
export FRESHER_DOCKER_IMAGE="${FRESHER_DOCKER_IMAGE:-fresher:local}"
