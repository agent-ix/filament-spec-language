# SPDX-License-Identifier: AGPL-3.0-or-later
# Use the root Makefile's sibling checkout defaults and exported environment.
.DEFAULT_GOAL := protocol-source-routes
include ../Makefile

.PHONY: protocol-source-routes
protocol-source-routes:
	cd "$(WORKSPACE_ROOT)" && cargo test --locked -p filament-spec-language --test it --features checked-handoff protocol_source_routes:: -- --test-threads=2
