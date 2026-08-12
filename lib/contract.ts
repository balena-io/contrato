/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import filter from 'lodash/filter';
import intersectionWith from 'lodash/intersectionWith';
import isEqual from 'lodash/isEqual';
import matches from 'lodash/matches';
import omit from 'lodash/omit';
import range from 'lodash/range';
import reduce from 'lodash/reduce';
import some from 'lodash/some';
import uniqWith from 'lodash/uniqWith';
import { Combination } from 'js-combinatorics';
import { compare, satisfies, valid, validRange } from 'semver';

import { isValid } from './json-schema';
import ObjectSet from './object-set';
import MatcherCache from './matcher-cache';
import { hashObject } from './hash';
import type { ContractObject, MatcherObject } from './types';
import { MATCHER } from './types';
import { compileContract } from './template';
import { build as buildVariants } from './variants';
import { getAll, build as buildChildrenTree } from './children-tree';
import { areSetsDisjoint } from './utils';

interface ContractChildrenMetadata {
	searchCache: MatcherCache;
	types: Set<string>;
	map: Record<string, Contract>;
	byType: Record<string, Set<string>>;
	byTypeSlug: Record<string, Record<string, Set<string>>>;
	typeMatchers: Record<string, Matcher>;
}

interface ContractRequirementsMetadata {
	matchers: Record<string, ObjectSet<Contract>>;
	types: Set<string>;
	compiled: ObjectSet<Contract>;
}

export interface ContractMetadata {
	children: ContractChildrenMetadata;
	requirements: ContractRequirementsMetadata;
}

export default class Contract {
	// Internal data about contract children and requirements
	protected $metadata: ContractMetadata;

	// The hash is lazily computed on the first `hash()` call and cached here.
	// It is a  native private field so it is not used when comparing contracts
	#hash: string | undefined;

	// The internal raw contract
	protected $raw: ContractObject;

	/**
	 * @summary Get a deep copy of the raw serializable contract
	 * @function
	 * @name module:contrato.Contract#raw
	 * @public
	 *
	 * @returns {ContractObject} a structural clone of the internal raw contract
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * console.log(contract.raw())
	 */
	public raw(): ContractObject {
		return structuredClone(this.$raw);
	}

	/**
	 * @summary A contract data structure
	 * @name Contract
	 * @memberof module:contrato
	 * @class
	 * @public
	 *
	 * @param {ContractObject} object - the contract plain object
	 *
	 * @example
	 * const contract = new Contract({
	 *   type: 'arch.sw',
	 *   name: 'armv7hf',
	 *   slug: 'armv7hf'
	 * })
	 */
	constructor(object: ContractObject) {
		this.$raw = object;
		this.$metadata = {
			children: {
				searchCache: new MatcherCache(),
				types: new Set(),
				map: {},
				byType: {},
				byTypeSlug: {},
				typeMatchers: {},
			},
			requirements: {
				matchers: {},
				types: new Set(),
				compiled: new ObjectSet(),
			},
		};

		for (const source of getAll(this.$raw.children)) {
			this.addChild(new Contract(source));
		}
		this.interpolate();
	}

	/**
	 * @summary Get the contract hash, computing it lazily if necessary
	 * @function
	 * @name module:contrato.Contract#hash
	 * @protected
	 *
	 * @description
	 * The hash is computed from the contract's raw object the first time
	 * it is requested, and cached afterwards. Operations that mutate the
	 * contract invalidate the cached hash, so it is recomputed on the next
	 * call.
	 *
	 * @returns {String} the contract hash
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * console.log(contract.hash())
	 */
	hash(): string {
		this.#hash ??= hashObject(this.$raw);
		return this.#hash;
	}

	/**
	 * @summary Re-build the contract's internal data structures
	 * @function
	 * @name module:contrato.Contract#rebuild
	 * @private
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.rebuild()
	 */
	private rebuild() {
		// Mutating the contract's raw object invalidates the cached hash,
		// which will be recomputed lazily on the next `hash()` call.
		this.#hash = undefined;
		const tree = buildChildrenTree(this.$metadata);
		if (Object.keys(tree).length > 0) {
			this.$raw.children = tree;
		}
		this.$metadata.requirements = {
			matchers: {},
			types: new Set(),
			compiled: new ObjectSet(),
		};
		/**
		 * @summary Register a leaf matcher so the contracts it references can
		 * be resolved by type later on (see getReferencedContracts)
		 * @function
		 * @private
		 *
		 * @param {Object} data - leaf matcher
		 *
		 * @example
		 * registerMatcher({
		 *   type: 'arch.sw',
		 *   slug: 'armv7hf'
		 * })
		 */
		const registerMatcher = (data: MatcherObject): void => {
			const matcher = Contract.createMatcher(data);
			this.$metadata.requirements.matchers[data.type] ??= new ObjectSet();
			this.$metadata.requirements.matchers[data.type].add(matcher, {
				id: matcher.hash(),
			});
			this.$metadata.requirements.types.add(data.type);
		};

		for (const conjunct of this.$raw.requires ?? []) {
			// A bare matcher is a single requirement on a contract type, while
			// an `or`/`not` operation wraps a list of sub-matchers.
			let matcher: Contract;
			let leaves: MatcherObject[];
			if ('type' in conjunct) {
				matcher = Contract.createMatcher(conjunct);
				leaves = [conjunct];
			} else {
				if (!('or' in conjunct) && !('not' in conjunct)) {
					throw new Error(
						'expected requirement to be a contract matcher or a `or/not` operation, got',
						conjunct,
					);
				}

				const [operation, disjuncts] =
					'or' in conjunct
						? (['or', conjunct.or] as const)
						: (['not', conjunct.not] as const);
				// Drop duplicate sub-matchers so an operation never carries the
				// same requirement twice.
				leaves = uniqWith(disjuncts, isEqual);
				matcher = Contract.createMatcher(leaves, { operation });
			}
			// Register every leaf so the contracts it references stay
			// discoverable by type.
			for (const leaf of leaves) {
				registerMatcher(leaf);
			}
			this.$metadata.requirements.compiled.add(matcher, {
				id: matcher.hash(),
			});
		}
	}
	/**
	 * @summary Interpolate the contract's template
	 * @function
	 * @name module:contrato.Contract#interpolate
	 * @protected
	 *
	 * @returns {Object} contract instance
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.interpolate()
	 */
	interpolate(): this {
		// TODO: Find a way to keep track of whether the contract
		// has already been fully templated, and if so, avoid
		// running this function.
		this.$raw = compileContract(this.$raw, {
			// Each contract is only templated using its own
			// properties, so here we prevent interpolations
			// on children using the master contract as a root.
			blacklist: new Set(['children']),
		});
		this.rebuild();
		return this;
	}
	/**
	 * @summary Get the contract version
	 * @function
	 * @name module:contrato.Contract#getVersion
	 * @public
	 *
	 * @returns {String} slug - contract version
	 *
	 * @example
	 * const contract = new Contract({
	 *   type: 'sw.os',
	 *   name: 'Debian Wheezy',
	 *   version: 'wheezy',
	 *   slug: 'debian'
	 * })
	 *
	 * console.log(contract.getVersion())
	 */
	getVersion(): string | undefined {
		return this.$raw.version;
	}
	/**
	 * @summary Get the contract slug
	 * @function
	 * @name module:contrato.Contract#getSlug
	 * @public
	 *
	 * @returns {String} slug - contract slug
	 *
	 * @example
	 * const contract = new Contract({
	 *   type: 'arch.sw',
	 *   name: 'armv7hf',
	 *   slug: 'armv7hf'
	 * })
	 *
	 * console.log(contract.getSlug())
	 */
	getSlug(): string | undefined {
		return this.$raw.slug;
	}
	/**
	 * @summary Get all the slugs this contract can be referenced with
	 * @function
	 * @name module:contrato.Contract#getAllSlugs
	 * @public
	 *
	 * @returns {Set} slugs
	 *
	 * @example
	 * const contract = new Contract({
	 *   type: 'hw.device-type',
	 *   name: 'Raspberry Pi',
	 *   slug: 'raspberrypi',
	 *   aliases: [ 'rpi', 'raspberry-pi' ]
	 * })
	 *
	 * console.log(contract.getAllSlugs())
	 * > Set { raspberrypi, rpi, raspberry-pi }
	 */
	getAllSlugs(): Set<string> {
		const slugs = new Set<string>(this.$raw.aliases);
		const thisSlug = this.getSlug();
		if (thisSlug != null) {
			slugs.add(thisSlug);
		}
		return slugs;
	}
	/**
	 * @summary Check if a contract has aliases
	 * @function
	 * @name module:contrato.Contract#hasAliases
	 * @public
	 *
	 * @returns {Boolean} whether the contract has aliases
	 *
	 * @example
	 * const contract = new Contract({
	 *   type: 'hw.device-type',
	 *   name: 'Raspberry Pi',
	 *   slug: 'raspberrypi',
	 *   aliases: [ 'rpi', 'raspberry-pi' ]
	 * })
	 *
	 * if (contract.hasAliases()) {
	 *   console.log('This contract has aliases')
	 * }
	 */
	hasAliases(): boolean {
		return this.$raw.aliases != null && this.$raw.aliases.length > 0;
	}
	/**
	 * @summary Get the contract canonical slug
	 * @function
	 * @name module:contrato.Contract#getCanonicalSlug
	 * @public
	 *
	 * @returns {String} slug - contract canonical slug or slug if canonical slug doesn't exist
	 *
	 * @example
	 * const contract = new Contract({
	 *   type: 'arch.sw',
	 *   name: 'armv7hf',
	 *   slug: 'armv7hf'
	 *   canonicalSlug: 'raspberry-pi'
	 * })
	 *
	 * console.log(contract.getCanonicalSlug())
	 */
	getCanonicalSlug(): string | undefined {
		// eslint-disable-next-line @typescript-eslint/prefer-nullish-coalescing
		return this.$raw.canonicalSlug || this.getSlug();
	}
	/**
	 * @summary Get the contract type
	 * @function
	 * @name module:contrato.Contract#getType
	 * @public
	 *
	 * @returns {String} type - contract type
	 *
	 * @example
	 * const contract = new Contract({
	 *   type: 'arch.sw',
	 *   name: 'armv7hf',
	 *   slug: 'armv7hf'
	 * })
	 *
	 * console.log(contract.getType())
	 */
	getType(): string {
		return this.$raw.type;
	}
	/**
	 * @summary Get a reference string for the contract
	 * @function
	 * @name module:contrato.Contract#getReferenceString
	 * @public
	 *
	 * @returns {String} reference string
	 *
	 * @example
	 * const contract = new Contract({
	 *   type: 'arch.sw',
	 *   name: 'armv7hf',
	 *   slug: 'armv7hf'
	 * })
	 *
	 * console.log(contract.getReferenceString())
	 */
	getReferenceString(): string {
		const slug = this.getSlug() ?? '';
		const version = this.getVersion();
		return version ? `${slug}@${version}` : slug;
	}
	/**
	 * @summary Return a JSON representation of a contract
	 * @function
	 * @name module:contrato.Contract#toJSON
	 * @public
	 *
	 * @returns {Object} JSON object
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * const object = contract.toJSON()
	 * console.log(JSON.stringify(object))
	 */
	toJSON(): ContractObject {
		// Ensure changes to the returned reference don't
		// accidentally mutate the contract's internal state
		return Object.assign({}, this.$raw);
	}
	/**
	 * @summary Add a child contract
	 * @function
	 * @name module:contrato.Contract#addChild
	 * @public
	 *
	 * @param {Object} contract - contract
	 * @param {Object} [options] - options
	 * @param {Boolean} [options.rebuild=true] - whether to re-build the parent contract
	 * @returns {Object} contract
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChild(new Contract({ ... }))
	 */
	addChild(contract: Contract, options: { rebuild?: boolean } = {}): this {
		const type = contract.getType();
		const childHash = contract.hash();
		if (this.$metadata.children.map[childHash]) {
			return this;
		}
		if (!this.$metadata.children.types.has(type)) {
			this.$metadata.children.types.add(type);
			this.$metadata.children.byType[type] = new Set();
			this.$metadata.children.byTypeSlug[type] = {};
		}
		for (const slug of contract.getAllSlugs()) {
			this.$metadata.children.byTypeSlug[type][slug] ??= new Set();
			this.$metadata.children.byTypeSlug[type][slug].add(childHash);
		}
		this.$metadata.children.map[childHash] = contract;
		this.$metadata.children.byType[type].add(childHash);
		this.$metadata.children.searchCache.resetType(type);
		if (options.rebuild ?? true) {
			this.rebuild();
		}
		return this;
	}
	/**
	 * @summary Remove a child contract
	 * @function
	 * @name module:contrato.Contract#removeChild
	 * @public
	 *
	 * @param {Object} contract - contract
	 * @returns {Object} parent contract
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 *
	 * const child = new Contract({ ... })
	 * contract.addChild(child)
	 * contract.removeChild(child)
	 */
	removeChild(contract: Contract): this {
		const type = contract.getType();
		const childHash = contract.hash();
		if (!this.$raw.children || !this.$metadata.children.map[childHash]) {
			return this;
		}
		Reflect.deleteProperty(this.$metadata.children.map, childHash);
		this.$metadata.children.byType[type].delete(childHash);
		if (this.$metadata.children.byType[type].size === 0) {
			Reflect.deleteProperty(this.$metadata.children.byType, type);
			this.$metadata.children.types.delete(type);
		}
		for (const slug of contract.getAllSlugs()) {
			this.$metadata.children.byTypeSlug[type][slug].delete(childHash);
			if (this.$metadata.children.byTypeSlug[type][slug].size === 0) {
				Reflect.deleteProperty(this.$metadata.children.byTypeSlug[type], slug);
			}
		}
		if (Object.keys(this.$metadata.children.byTypeSlug[type]).length === 0) {
			Reflect.deleteProperty(this.$metadata.children.byTypeSlug, type);
		}
		this.$metadata.children.searchCache.resetType(contract.getType());
		this.rebuild();
		return this;
	}
	/**
	 * @summary Add a set of children contracts to the contract
	 * @function
	 * @name module:contrato.Contract#addChildren
	 * @public
	 *
	 * @description
	 * This is a utility method over `.addChild()`.
	 *
	 * @param {Object[]} contracts - contracts
	 * @returns {Object} contract
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChildren([
	 *   new Contract({ ... }),
	 *   new Contract({ ... }),
	 *   new Contract({ ... })
	 * ])
	 */
	addChildren(contracts: Contract[] = []): this {
		if (!contracts) {
			return this;
		}
		for (const contract of contracts) {
			this.addChild(contract, {
				// For performance reasons. If this is set to true,
				// then we would re-build the contract N times, where
				// N is the number of contracts passed to this function.
				// Intead, we can prevent re-building and only do it
				// once when the function completes.
				rebuild: false,
			});
		}
		this.rebuild();
		return this;
	}
	/**
	 * @summary Recursively get the list of types known children contract types
	 * @function
	 * @name module:contrato.Contract#getChildrenTypes
	 * @public
	 *
	 * @returns {Set} types
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChildren([ { ... }, { ... } ])
	 * console.log(contract.getChildrenTypes())
	 */
	getChildrenTypes(): Set<string> {
		const types = new Set<string>(this.$metadata.children.types);
		for (const contract of this.getChildren()) {
			for (const type of contract.getChildrenTypes()) {
				types.add(type);
			}
		}
		return types;
	}
	/**
	 * @summary Get a single child by its hash
	 * @function
	 * @name module:contrato.Contract#getChildByHash
	 * @public
	 *
	 * @param {String} childHash - child contract hash
	 * @returns {(Object|Undefined)} child
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChildren([ ... ])
	 *
	 * const child = contract.getChildByHash('xxxxxxx')
	 *
	 * if (child) {
	 *   console.log(child)
	 * }
	 */
	getChildByHash(childHash: string): Contract | undefined {
		return this.$metadata.children.map[childHash];
	}
	/**
	 * @summary Recursively get a set of children contracts
	 * @function
	 * @name module:contrato.Contract#getChildren
	 * @public
	 *
	 * @param {Object} [options] - options
	 * @param {Set} [options.types] - children types (all by default)
	 * @returns {Object[]} children
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * const children = contract.getChildren({
	 *   types: new Set([ 'arch.sw' ])
	 * })
	 *
	 * for (const child of children) {
	 *   console.log(child)
	 * }
	 */
	getChildren(options: { types?: Set<string> } = {}): Contract[] {
		const contracts: Contract[] = [];
		for (const contract of Object.values(this.$metadata.children.map)) {
			if (!options.types || options.types.has(contract.$raw.type)) {
				contracts.push(contract);
			}
			contracts.push(...contract.getChildren(options));
		}
		return contracts;
	}
	/**
	 * @summary Get all the children contracts of a specific type
	 * @function
	 * @name module:contrato.Contract#getChildrenByType
	 * @public
	 *
	 * @param {String} type - contract type
	 * @returns {Object[]} children
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChildren([ ... ])
	 * const children = container.getChildrenByType('sw.os')
	 *
	 * children.forEach((child) => {
	 *   console.log(child)
	 * })
	 */
	getChildrenByType(type: string): Contract[] {
		this.$metadata.children.typeMatchers[type] ??= Contract.createMatcher({
			type,
		});
		return this.findChildren(this.$metadata.children.typeMatchers[type]);
	}
	/**
	 * @summary Recursively find children using a matcher contract
	 * @function
	 * @name module:contrato.Contract#findChildren
	 * @public
	 *
	 * @param {Object} matcher - matcher contract
	 * @returns {Object[]} children
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChildren([ ... ])
	 *
	 * const children = contract.findChildren(Contract.createMatcher({
	 *   type: 'sw.os',
	 *   slug: 'debian'
	 * }))
	 *
	 * children.forEach((child) => {
	 *   console.log(child)
	 * })
	 */
	findChildrenWithCapabilities(matcher: Contract): Contract[] {
		if (!matcher.$raw) {
			return [];
		}
		const results: Contract[] = [];
		for (const contract of this.getChildren().concat([this])) {
			// We need to omit the slug from the matcher object, otherwise
			// matchers that use an alias as a slug will never match the
			// structure of the actual contract.
			// Notice we do use the slug key separately, in order to obtain
			// the list of hashes we should check against.
			const match = matches(omit(matcher.$raw.data, ['slug', 'version']));
			const versionMatch = matcher.$raw.data?.version;
			if (contract.$raw.provides) {
				for (const capability of contract.$raw.provides) {
					if (match(capability)) {
						if (versionMatch) {
							if (
								capability.version != null &&
								valid(capability.version) &&
								validRange(versionMatch)
							) {
								if (satisfies(capability.version, versionMatch)) {
									results.push(contract);
								}
							} else if (isEqual(capability.version, versionMatch)) {
								results.push(contract);
							}
							continue;
						}
						results.push(contract);
					}
				}
			}
		}
		return uniqWith(results, isEqual);
	}
	/**
	 * @summary Recursively find children using a matcher contract
	 * @function
	 * @name module:contrato.Contract#findChildren
	 * @public
	 *
	 * @param {Object} matcher - matcher contract
	 * @returns {Object[]} children
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChildren([ ... ])
	 *
	 * const children = contract.findChildren(Contract.createMatcher({
	 *   type: 'sw.os',
	 *   slug: 'debian'
	 * }))
	 *
	 * children.forEach((child) => {
	 *   console.log(child)
	 * })
	 */
	findChildren(matcher: Contract): Contract[] {
		if (!(matcher instanceof Matcher)) {
			throw new Error('expected contract to be a Matcher instance');
		}
		const type = matcher.getMatchedType();
		if (type == null || !this.getChildrenTypes().has(type)) {
			return [];
		}
		const cache = this.$metadata.children.searchCache.get(matcher);
		if (cache) {
			return cache;
		}
		const results: Contract[] = [];
		const slug = matcher.$raw.data.slug;
		for (const contract of this.getChildren().concat([this])) {
			if (!contract.$metadata.children.types.has(type)) {
				continue;
			}
			// We need to omit the slug from the matcher object, otherwise
			// matchers that use an alias as a slug will never match the
			// structure of the actual contract.
			// Notice we do use the slug key separately, in order to obtain
			// the list of hashes we should check against.
			const match = matches(omit(matcher.$raw.data, ['slug', 'version']));
			const versionMatch = matcher.$raw.data.version;
			const hashes = slug
				? (contract.$metadata.children.byTypeSlug[type][slug] ?? new Set())
				: contract.$metadata.children.byType[type];
			// Means that we are matching just the type
			if (Object.keys(matcher.$raw.data).length === 1) {
				for (const childHash of hashes) {
					const child = contract.getChildByHash(childHash);
					if (!child) {
						throw new Error('Error retrieving child');
					}
					results.push(child);
				}
			} else {
				for (const childHash of hashes) {
					const child = contract.getChildByHash(childHash);
					if (child && match(child.$raw)) {
						if (versionMatch) {
							if (
								child.$raw.version != null &&
								valid(child.$raw.version) &&
								validRange(versionMatch)
							) {
								if (satisfies(child.$raw.version, versionMatch)) {
									results.push(child);
								}
							} else if (isEqual(child.$raw.version, versionMatch)) {
								results.push(child);
							}
							continue;
						}
						results.push(child);
					}
				}
			}
		}
		this.$metadata.children.searchCache.add(matcher, results);
		return results;
	}
	/**
	 * @summary Get all possible combinations from a type of children contracts
	 * @function
	 * @name module:contrato.Contract#getChildrenCombinations
	 * @public
	 *
	 * @description
	 * Note that the client is responsible for evaluating that the
	 * combination of contracts is valid with regards to requirements,
	 * conflicts, etc. This function simply returns all the possible
	 * combinations without any further checks.
	 *
	 * The combinations output by this function is a plain list of
	 * contracts from which you can create a contract, or any other
	 * application specific data structure.
	 *
	 * @param {Object} options - options
	 * @param {String} options.type - contract type
	 * @param {Number} options.from - number of contracts per combination (from)
	 * @param {Number} options.to - number of contracts per combination (to)
	 * @returns {Array[]} combinations
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChildren([
	 *   new Contract({
	 *     name: 'Debian Wheezy',
	 *     version: 'wheezy',
	 *     slug: 'debian',
	 *     type: 'sw.os'
	 *   }),
	 *   new Contract({
	 *     name: 'Debian Jessie',
	 *     version: 'jessie',
	 *     slug: 'debian',
	 *     type: 'sw.os'
	 *   }),
	 *   new Contract({
	 *     name: 'Fedora 25',
	 *     version: '25',
	 *     slug: 'fedora',
	 *     type: 'sw.os'
	 *   })
	 * ])
	 *
	 * const combinations = contract.getChildrenCombinations({
	 *   type: 'sw.os',
	 *   from: 2,
	 *   to: 2
	 * })
	 *
	 * console.log(combinations)
	 * > [
	 * >   [
	 * >     new Contract({
	 * >       name: 'Debian Wheezy',
	 * >       version: 'wheezy',
	 * >       slug: 'debian',
	 * >       type: 'sw.os'
	 * >     }),
	 * >     new Contract({
	 * >       name: 'Debian Jessie',
	 * >       version: 'jessie',
	 * >       slug: 'debian',
	 * >       type: 'sw.os'
	 * >     })
	 * >   ],
	 * >   [
	 * >     new Contract({
	 * >       name: 'Debian Wheezy',
	 * >       version: 'wheezy',
	 * >       slug: 'debian',
	 * >       type: 'sw.os'
	 * >     }),
	 * >     new Contract({
	 * >       name: 'Fedora 25',
	 * >       version: '25',
	 * >       slug: 'fedora',
	 * >       type: 'sw.os'
	 * >     })
	 * >   ],
	 * >   [
	 * >     new Contract({
	 * >       name: 'Debian Jessie',
	 * >       version: 'jessie',
	 * >       slug: 'debian',
	 * >       type: 'sw.os'
	 * >     }),
	 * >     new Contract({
	 * >       name: 'Fedora 25',
	 * >       version: '25',
	 * >       slug: 'fedora',
	 * >       type: 'sw.os'
	 * >     })
	 * >   ]
	 * > ]
	 */
	getChildrenCombinations(options: {
		type: string;
		from?: number;
		to?: number;
		[index: string]: any;
	}): Contract[][] {
		let contracts = this.getChildrenByType(options.type);
		// eslint-disable-next-line @typescript-eslint/prefer-nullish-coalescing
		const cardinality = options['cardinality'] || options;
		if (options['filter']) {
			contracts = contracts.filter((con) => {
				return isValid(options['filter'], con.$raw);
			});
		}
		if (contracts.length > 0) {
			if (options['version']) {
				if (isEqual(options['version'], 'latest')) {
					contracts = contracts.filter((c) => c.getVersion() != null);
					contracts.sort((left, right) => {
						return compare(right.getVersion()!, left.getVersion()!);
					});
					contracts = contracts.slice(
						0,
						Math.min(contracts.length, cardinality.to),
					);
				} else {
					contracts = contracts.filter((con) => {
						const v = con.getVersion();
						return v != null && satisfies(v, options['version']);
					});
				}
			}
		}
		if (contracts.length < cardinality.from) {
			throw new Error(
				`Invalid cardinality: ${cardinality.from} to ${cardinality.to}. ` +
					`The number of ${options.type} contracts in ` +
					`the universe is ${contracts.length}`,
			);
		}
		if (cardinality.from > cardinality.to) {
			throw new Error(
				`Invalid cardinality: ${cardinality.from} to ${cardinality.to}. ` +
					'The starting point is greater than the ending point',
			);
		}
		const rang = range(
			cardinality.from,
			Math.min(cardinality.to, contracts.length) + 1,
		);
		return rang.flatMap((tcardinality) => {
			return new Combination(contracts, tcardinality).toArray();
		});
	}
	/**
	 * @summary Recursively get the list of referenced contracts
	 * @function
	 * @name module:contrato.Contract#getReferencedContracts
	 * @public
	 *
	 * @param {Object} options - options
	 * @param {Object} options.from - contract to resolve external contracts from
	 * @param {Set} options.types - types to consider
	 * @returns {Object[]} referenced contracts
	 *
	 * @example
	 * const universe = new Contract({ ... })
	 * universe.addChildren([ ... ])
	 *
	 * const contract = new Contract({ ... })
	 * for (const reference of contract.getReferencedContracts({
	 *   types: new Set([ 'arch.sw' ]),
	 *   from: universe
	 * })) {
	 *   console.log(reference.toJSON())
	 * }
	 */
	getReferencedContracts(options: { types: Set<string>; from: Contract }): {
		[index: string]: Contract[];
	} {
		const references: { [index: string]: Contract[] } = {};
		for (const type of options.types) {
			if (!this.$metadata.requirements.types.has(type)) {
				continue;
			}
			references[type] = [];
			const matchers = this.$metadata.requirements.matchers[type].getAll();
			for (const matcher of matchers) {
				for (const find of options.from.findChildren(matcher)) {
					references[find.getType()].push(find);
					const nested = find.getReferencedContracts(options);
					for (const nestedType of Object.keys(nested)) {
						for (const contract of nested[nestedType]) {
							references[nestedType].push(contract);
						}
					}
				}
			}
		}
		return references;
	}
	/**
	 * @summary Get the children cross referenced contracts
	 * @function
	 * @name module:contrato.Contract#getChildrenCrossReferencedContracts
	 * @public
	 *
	 * @param {Object} options - options
	 * @param {Object} options.from - contract to resolve external contracts from
	 * @param {Set} options.types - types to consider
	 * @returns {Object[]} children cross referenced contracts
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 *
	 * contract.addChildren([
	 *   new Contract({
	 *     type: 'arch.sw',
	 *     slug: 'armv7hf',
	 *     name: 'armv7hf'
	 *   }),
	 *   new Contract({
	 *     type: 'sw.os',
	 *     slug: 'raspbian',
	 *     requires: [
	 *       {
	 *         or: [
	 *           {
	 *             type: 'arch.sw',
	 *             slug: 'armv7hf'
	 *           },
	 *           {
	 *             type: 'arch.sw',
	 *             slug: 'rpi'
	 *           }
	 *         ]
	 *       }
	 *     ]
	 *   }),
	 *   new Contract({
	 *     type: 'sw.stack',
	 *     slug: 'nodejs',
	 *     requires: [
	 *       {
	 *         type: 'arch.sw',
	 *         slug: 'armv7hf'
	 *       }
	 *     ]
	 *   })
	 * ])
	 *
	 * const references = contract.getChildrenCrossReferencedContracts({
	 *   from: contract,
	 *   types: new Set([ 'arch.sw' ])
	 * })
	 *
	 * console.log(references)
	 * > [
	 * >   Contract {
	 * >     type: 'arch.sw',
	 * >     slug: 'armv7hf',
	 * >     name: 'armv7hf'
	 * >   }
	 * > ]
	 */
	getChildrenCrossReferencedContracts(options: {
		types: Set<string>;
		from: Contract;
	}): Contract[] {
		const result: { [index: string]: Contract[][] } = {};
		for (const contract of this.getChildren()) {
			const references = contract.getReferencedContracts(options);
			for (const type of Object.keys(references)) {
				if (!result[type]) {
					result[type] = [];
				}
				result[type].push(references[type]);
			}
		}
		return reduce(
			result,
			(accumulator, value) => {
				return accumulator.concat(intersectionWith(...value, Contract.isEqual));
			},
			[] as Contract[],
		);
	}

	private isRequirementSatisfied(
		requirement: Contract,
		options: { types?: Set<string> } = {},
	): boolean {
		// Utilities
		const shouldEvaluateType = (type: string) =>
			options.types ? options.types.has(type) : true;

		/**
		 * @summary Check if a matcher is satisfied
		 * @function
		 * @public
		 *
		 * @param {Object} matcher - matcher contract
		 * @returns {Boolean} whether the matcher is satisfied
		 *
		 * @example
		 * const matcher = Contract.createMatcher({
		 *   type: 'sw.os',
		 *   slug: 'debian'
		 * })
		 *
		 * if (hasMatch(matcher)) {
		 *   console.log('This matcher is satisfied!')
		 * }
		 */
		const hasMatch = (matcher: Contract): boolean => {
			// TODO: Write a function similar to findContracts
			// that stops as soon as it finds one match
			return (
				this.findChildren(matcher).length > 0 ||
				this.findChildrenWithCapabilities(matcher).length > 0
			);
		};

		if (requirement.$raw.operation === 'or') {
			// (3.1) Note that we should only consider disjuncts
			// of types we are allowed to check. We can make
			// such transformation here, so we can then consider
			// the disjunction as fulfilled if there are no
			// remaining disjuncts.
			const disjuncts = filter(
				requirement.$raw.data as MatcherObject[],
				(disjunct) => shouldEvaluateType(disjunct.type),
			);
			// (3.2) An empty disjuction means that this particular
			// requirement is fulfilled, so we can carry on.
			// A disjunction naturally contains a list of further
			// requirements we need to check for. If at least one
			// of the members is fulfilled, we can proceed with
			// next requirement.
			if (
				disjuncts.length === 0 ||
				disjuncts.some((disjunct) => hasMatch(Contract.createMatcher(disjunct)))
			) {
				return true;
			}
			// (3.3) If no members were fulfilled, then we know
			// that this requirement was not fullfilled, so it will be returned
			return false;
		} else if (requirement.$raw.operation === 'not') {
			// (3.4) Note that we should only consider disjuncts
			// of types we are allowed to check. We can make
			// such transformation here, so we can then consider
			// the disjunction as fulfilled if there are no
			// remaining disjuncts.
			// (3.5) We fail the requirement if the set of negated
			// disjuncts is not empty, and we have at least one of
			// them in the context.
			if (
				some(requirement.$raw.data as MatcherObject[], (disjunct) => {
					return (
						shouldEvaluateType(disjunct.type) &&
						hasMatch(Contract.createMatcher(disjunct))
					);
				})
			) {
				return false;
			}
			return true;
		}
		// (4) If we should evaluate this requirement and it is not fullfilled
		// it will be returned
		if (
			shouldEvaluateType(requirement.$raw.data.type) &&
			!hasMatch(requirement)
		) {
			return false;
		}

		return true;
	}

	/**
	 * @summary Get a list of child requirements that are not satisfied by this contract
	 * @function
	 * @name module:contrato.Contract#satisfiesChildContract
	 * @public
	 *
	 * @param {Object} contract - child contract
	 * @param {Object} [options] - options
	 * @param {Set} [options.types] - the types to consider (all by default)
	 * @returns list of unsatisfied requirements
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChildren([
	 *   new Contract({
	 *     type: 'sw.os',
	 *     name: 'Debian Wheezy',
	 *     version: 'wheezy',
	 *     slug: 'debian'
	 *   }),
	 *   new Contract({
	 *     type: 'sw.os',
	 *     name: 'Fedora 25',
	 *     version: '25',
	 *     slug: 'fedora'
	 *   })
	 * ])
	 *
	 * const child = new Contract({
	 *   type: 'sw.stack',
	 *   name: 'Node.js',
	 *   version: '4.8.0',
	 *   slug: 'nodejs',
	 *   requires: [
	 *     {
	 *       or: [
	 *         {
	 *           type: 'sw.os',
	 *           slug: 'debian'
	 *         },
	 *         {
	 *           type: 'sw.os',
	 *           slug: 'fedora'
	 *         }
	 *       ]
	 *     }
	 *   ]
	 * })
	 *
	 * if (contract.satisfiesChildContract(child)) {
	 *   console.log('The child contract is satisfied!')
	 * }
	 */
	getNotSatisfiedChildRequirements(
		contract: Contract,
		options: { types?: Set<string> } = {},
	) {
		const conjuncts: Contract[] = contract.$metadata.requirements.compiled
			.getAll()
			.concat(
				contract
					.getChildren()
					.flatMap((child) => child.$metadata.requirements.compiled.getAll()),
			);
		// (1) If the top level list of conjuncts is empty,
		// then we can assume the requirements are fulfilled
		// and stop without doing any further computations.
		if (conjuncts.length === 0) {
			return [];
		}

		// (2) The requirements are specified as a list of objects,
		// so lets iterate through those.
		// This function uses a for loop instead of a more functional
		// construct for performance reasons, given that we can freely
		// break out of the loop as soon as possible.
		return conjuncts
			.filter((conjunct) => !this.isRequirementSatisfied(conjunct, options))
			.map((conjunct) => conjunct.$raw.data);
		// (5) If we reached this far, then it means that all the
		// requirements were checked, and they were all satisfied,
		// so this is good to go!
	}
	/**
	 * @summary Check if a child contract is satisfied when applied to this contract
	 * @function
	 * @name module:contrato.Contract#satisfiesChildContract
	 * @public
	 *
	 * @param {Object} contract - child contract
	 * @param {Object} [options] - options
	 * @param {Set} [options.types] - the types to consider (all by default)
	 * @returns {Boolean} whether the contract is satisfied
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChildren([
	 *   new Contract({
	 *     type: 'sw.os',
	 *     name: 'Debian Wheezy',
	 *     version: 'wheezy',
	 *     slug: 'debian'
	 *   }),
	 *   new Contract({
	 *     type: 'sw.os',
	 *     name: 'Fedora 25',
	 *     version: '25',
	 *     slug: 'fedora'
	 *   })
	 * ])
	 *
	 * const child = new Contract({
	 *   type: 'sw.stack',
	 *   name: 'Node.js',
	 *   version: '4.8.0',
	 *   slug: 'nodejs',
	 *   requires: [
	 *     {
	 *       or: [
	 *         {
	 *           type: 'sw.os',
	 *           slug: 'debian'
	 *         },
	 *         {
	 *           type: 'sw.os',
	 *           slug: 'fedora'
	 *         }
	 *       ]
	 *     }
	 *   ]
	 * })
	 *
	 * if (contract.satisfiesChildContract(child)) {
	 *   console.log('The child contract is satisfied!')
	 * }
	 */
	satisfiesChildContract(
		contract: Contract,
		options: { types?: Set<string> } = {},
	): boolean {
		const conjuncts: Contract[] = contract.$metadata.requirements.compiled
			.getAll()
			.concat(
				contract
					.getChildren()
					.flatMap((child) => child.$metadata.requirements.compiled.getAll()),
			);

		// (1) If the top level list of conjuncts is empty,
		// then we can assume the requirements are fulfilled
		// and stop without doing any further computations.
		if (conjuncts.length === 0) {
			return true;
		}

		// (2) The requirements are specified as a list of objects,
		// so lets iterate through those.
		// This function uses a for loop instead of a more functional
		// construct for performance reasons, given that we can freely
		// break out of the loop as soon as possible.
		for (const conjunct of conjuncts) {
			// (3-4) stop looking if an unsatisfied requirement is found
			if (!this.isRequirementSatisfied(conjunct, options)) {
				return false;
			}
		}
		// (5) If we reached this far, then it means that all the
		// requirements were checked, and they were all satisfied,
		// so this is good to go!
		return true;
	}

	/**
	 * @summary Check if the contract children are satisfied
	 * @function
	 * @name module:contrato.Contract#areChildrenSatisfied
	 * @public
	 *
	 * @param {Object} [options] - options
	 * @param {Set} [options.types] - the types to consider (all by default)
	 * @returns {Boolean} whether the children are satisfied
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChildren([ ... ])
	 *
	 * if (contract.areChildrenSatisfied({
	 *   types: new Set([ 'sw.arch' ])
	 * })) {
	 *   console.log('This contract has all sw.arch requirements satisfied')
	 * }
	 */
	areChildrenSatisfied(options: { types?: Set<string> } = {}): boolean {
		for (const contract of this.getChildren()) {
			// The contract object keeps track of which contract
			// types the contract references in the requirements.
			// If we specified a set of types and we know this
			// contract is not interested in them, then we can
			// continue and avoid traversing through all the
			// requirements in vain.
			if (
				options.types &&
				areSetsDisjoint(options.types, contract.$metadata.requirements.types)
			) {
				continue;
			}
			if (
				!this.satisfiesChildContract(contract, {
					types: options.types,
				})
			) {
				return false;
			}
		}
		return true;
	}

	/**
	 *
	 * @param {@summary} options
	 */
	getAllNotSatisfiedChildRequirements(
		options: { types?: Set<string> } = {},
	): any[] {
		let requirements: any[] = [];
		for (const contract of this.getChildren()) {
			// The contract object keeps track of which contract
			// types the contract references in the requirements.
			// If we specified a set of types and we know this
			// contract is not interested in them, then we can
			// continue and avoid traversing through all the
			// requirements in vain.
			if (
				options.types &&
				areSetsDisjoint(options.types, contract.$metadata.requirements.types)
			) {
				requirements = requirements.concat(
					contract.$metadata.requirements.compiled
						.getAll()
						.map((c) => c.$raw.data),
				);
				continue;
			}
			const contractRequirements = this.getNotSatisfiedChildRequirements(
				contract,
				{
					types: options.types,
				},
			);
			requirements = requirements.concat(contractRequirements);
		}
		return requirements;
	}
	/**
	 * @summary Create a matcher contract object
	 * @function
	 * @static
	 * @name module:contrato.Contract.createMatcher
	 * @protected
	 *
	 * @param {(Object|Object[])} obj - a single matcher, or the list of
	 * sub-matchers when building an `or`/`not` operation
	 * @param {Object} [options] - options
	 * @param {String} [options.operation] - the matcher's operation
	 * @returns {Object} matcher contract
	 *
	 * @example
	 * const matcher = Contract.createMatcher({
	 *   type: 'arch.sw',
	 *   slug: 'armv7hf'
	 * })
	 */
	static createMatcher(
		obj: MatcherObject | MatcherObject[],
		options: { operation?: 'or' | 'not' } = {},
	): Matcher {
		// Reject matchers carrying anything other than these fields; further
		// matching criteria belong under `data`.
		const fields = new Set(['type', 'slug', 'version', 'data'] satisfies Array<
			keyof MatcherObject
		>);
		for (const matcher of Array.isArray(obj) ? obj : [obj]) {
			const unknownProp = Object.keys(matcher).find(
				(key) => !fields.has(key as keyof MatcherObject),
			);
			if (unknownProp) {
				throw new Error(
					`unknown field \`${unknownProp}\`, expected one of ${Array.from(
						fields,
					)
						.map((field) => `\`${field}\``)
						.join(', ')}`,
				);
			}
		}
		return new Matcher({
			type: MATCHER,
			operation: options.operation,
			data: obj,
		});
	}
	/**
	 * @summary Check if two contracts are equal
	 * @function
	 * @static
	 * @name module:contrato.Contract.isEqual
	 * @public
	 *
	 * @param {Object} contract1 - a contract
	 * @param {Object} contract2 - a contract
	 * @returns {Boolean} whether the contracts are equal
	 *
	 * @example
	 * const contract1 = new Contract({ ... })
	 * const contract2 = new Contract({ ... })
	 *
	 * if (Contract.isEqual(contract1, contract2)) {
	 *   console.log('These contracts are equal')
	 * }
	 */
	static isEqual(contract1: Contract, contract2: Contract): boolean {
		return contract1 === contract2 || contract1.hash() === contract2.hash();
	}
	/**
	 * @summary Build a source contract
	 * @function
	 * @static
	 * @name module:contrato.Contract.build
	 * @public
	 *
	 * @param {Object} source - source contract
	 * @returns {Object[]} built contracts
	 *
	 * @example
	 * const contracts = Contract.build({
	 *   name: 'debian {{version}}',
	 *   slug: 'debian',
	 *   type: 'sw.os',
	 *   variants: [
	 *     { version: 'wheezy' },
	 *     { version: 'jessie' },
	 *     { version: 'sid' }
	 *   ]
	 * })
	 *
	 * contracts.forEach((contract) => {
	 *   if (contract instanceof Contract) {
	 *     console.log('This is a built contract')
	 *   }
	 * })
	 */
	static build(source: ContractObject): Contract[] {
		const rawContracts = buildVariants(source);
		return rawContracts.reduce<Contract[]>((accumulator, variant) => {
			const aliases = Array.isArray(variant['aliases'])
				? variant['aliases']
				: [];
			const obj = omit(variant, ['aliases']) as ContractObject;
			const contracts = aliases.map((alias) => {
				return new Contract(
					Object.assign({}, obj, {
						canonicalSlug: obj['slug'],
						slug: alias,
					}),
				);
			});
			contracts.push(new Contract(obj));
			return accumulator.concat(contracts);
		}, []);
	}
}

/**
 * @summary A matcher contract
 * @name Matcher
 * @class
 * @protected
 *
 * @description
 * A contract of type `meta.matcher` whose `data` describes the contracts
 * to look for. Instances are created via {@link Contract.createMatcher}.
 */
export class Matcher extends Contract {
	/**
	 * @summary Get the contract type matched by this matcher
	 * @function
	 * @name module:contrato.Matcher#getMatchedType
	 * @protected
	 *
	 * @description
	 * Operation matchers (e.g. `or`, `not`) group other matchers instead of
	 * describing a single contract, so they have no matched type.
	 *
	 * @returns {String|undefined} the matched contract type, if any
	 *
	 * @example
	 * const matcher = Contract.createMatcher({ type: 'sw.os', slug: 'debian' })
	 * console.log(matcher.getMatchedType())
	 * > 'sw.os'
	 */
	getMatchedType(): string | undefined {
		return this.$raw.data.type;
	}
}
