/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

export const CONTEXT = 'meta.context';
export const UNIVERSE = 'meta.universe';
export const BLUEPRINT = 'meta.blueprint';

export interface Asset {
	/** The URL where the asset can be retrieved */
	url: string;

	/** Optional human-readable name for the asset */
	name?: string;

	/** Optional checksum for integrity verification */
	checksum?: string;

	/** Optional checksum algorithm (e.g., `"sha256"`) */
	checksumType?: string;

	/** Additional top-level fields, preserved for round-trip fidelity. */
	[key: string]: unknown;
}

/// A matcher that references contracts by type and optional additional criteria.
///
/// Used as requirement targets: what a contract needs from its context. Per the
/// CUE spec, additional matching criteria should be placed in `data`, not as
/// top-level fields.
export interface MatcherObject {
	/** The contract type to match against. */
	type: string;

	/** Unique identifier of the contract within its type. */
	slug?: string;

	/** Version string: semver or a plain identifier (e.g. `wheezy`). */
	version?: string;

	/** Free-form data specific to the contract type. */
	data?: any;
}

export type ContractRequirement =
	MatcherObject | { or: MatcherObject[] } | { not: MatcherObject[] };

/**
 * Children contracts, either as a flat list or as the nested
 * `{ type: { slug: contract } }` tree. Both forms are accepted as input;
 * contracts always serialize back out as the nested tree.
 */
export type ChildrenTree = Record<string, unknown> | ContractObject[];

interface PartialContract {
	/** Unique identifier of the contract within its type. */
	slug?: string;

	/** Version string: semver or a plain identifier (e.g. `wheezy`). */
	version?: string;

	/** Human-readable name. */
	name?: string;

	/** Human-readable description. */
	description?: string;

	/** Alternative slugs this contract can be referenced by. */
	aliases?: string[];

	/** Free-form data specific to the contract type. */
	data?: any;

	/** Named assets attached to the contract. */
	assets?: Record<string, Asset>;

	/** Requirements that must be satisfied for this contract. */
	requires?: ContractRequirement[];

	/** Nested variants, deep-merged with the base contract during expansion. */
	variants?: PartialContract[];

	/**
	 * Children contracts. This is also how a contract declares the
	 * capabilities it makes available to its context.
	 */
	children?: ChildrenTree;
}

export interface ContractObject extends PartialContract {
	/** The contract type, e.g. `sw.os` or `hw.device-type`. */
	type: string;

	/** Canonical slug an alias maps back to. */
	canonicalSlug?: string;

	/** Additional top-level fields, preserved for round-trip fidelity. */
	[key: string]: unknown;
}

/**
 * Free-form blueprint layout; each entry maps a contract type to a selector
 * descriptor — either a cardinality shorthand or an object carrying
 * `cardinality`, `filter`, `type` and `version`. Entries are duck-typed at
 * construction time, hence `any`.
 */
export type BlueprintLayout = Record<string, any>;

/**
 * A blueprint contract: a contract of type `meta.blueprint` carrying a
 * skeleton. The layout is parsed and kept separately, not on the raw object,
 * since selectors may carry functions that cannot be structurally cloned.
 */
export type BlueprintObject = ContractObject & {
	type?: typeof BLUEPRINT;
	skeleton: ContractObject;
};
