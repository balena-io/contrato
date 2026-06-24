/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

export const CONTEXT = 'meta.context';
export const UNIVERSE = 'meta.universe';
export const MATCHER = 'meta.matcher';
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
}

export type ChildrenTree = Record<string, unknown>;

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
	requires?: Array<Record<string, unknown>>;

	/** Capabilities this contract provides */
	provides?: ContractCapability[];

	/** Nested variants, deep-merged with the base contract during expansion. */
	variants?: PartialContract[];

	/** Children contracts as a nested `{ type: { slug: contract } }` tree. */
	children?: ChildrenTree;
}

export interface ContractCapability extends PartialContract {
	/** The contract type, e.g. `sw.os` or `hw.device-type`. */
	type: string;
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
 * A blueprint contract: a contract of type `meta.blueprint` carrying a layout
 * and an optional skeleton.
 */
export type BlueprintObject = ContractObject & {
	type?: typeof BLUEPRINT;
	layout?: BlueprintLayout;
	skeleton: ContractObject;
};
