/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import concat from 'lodash/concat';
import defaults from 'lodash/defaults';
import filter from 'lodash/filter';
import first from 'lodash/first';
import flatMap from 'lodash/flatMap';
import intersectionWith from 'lodash/intersectionWith';
import isEqual from 'lodash/isEqual';
import keys from 'lodash/keys';
import map from 'lodash/map';
import matches from 'lodash/matches';
import omit from 'lodash/omit';
import partial from 'lodash/partial';
import range from 'lodash/range';
import reduce from 'lodash/reduce';
import some from 'lodash/some';
import uniqWith from 'lodash/uniqWith';
import { bigCombination } from 'js-combinatorics';
import { compare, satisfies, valid, validRange } from 'semver';

import { isValid } from './json-schema';
import ObjectSet from './object-set';
import MatcherCache from './matcher-cache';
import { hashObject } from './hash';
import type { ContractObject } from './types';
import { MATCHER } from './types';
import { compileContract } from './template';
import { build as buildVariants } from './variants';
import { build as buildChildrentree } from './children-tree';
import { getAll } from './children-tree';
import { areSetsDisjoint } from './utils';
import { forEach } from 'lodash';

export default class Contract {
	metadata: any;
	raw: ContractObject;
	/**
	 * @summary A contract data structure
	 * @name Contract
	 * @memberof module:contrato
	 * @class
	 * @public
	 *
	 * @param {ContractObject} object - the contract plain object
	 * @param {Object} [options] - options
	 * @param {Boolean} [options.hash=true] - whether to hash the contract
	 *
	 * @example
	 * const contract = new Contract({
	 *   type: 'arch.sw',
	 *   name: 'armv7hf',
	 *   slug: 'armv7hf'
	 * })
	 */
	constructor(object: ContractObject, options: object = {}) {
		this.raw = object;
		this.metadata = {
			children: {
				searchCache: new MatcherCache(),
				types: new Set(),
				map: {},
				byType: {},
				byTypeSlug: {},
				typeMatchers: {},
			},
		};
		for (const source of getAll(this.raw.children)) {
			this.addChild(new Contract(source));
		}
		this.interpolate({
			rehash: false,
		});
		defaults(options, {
			hash: true,
		});
		if ((options as any).hash) {
			this.hash();
		}
	}
	/**
	 * @summary Re-hash the contract
	 * @function
	 * @name module:contrato.Contract#hash
	 * @protected
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.hash()
	 */
	hash() {
		this.metadata.hash = hashObject(this.raw);
	}
	/**
	 * @summary Re-build the contract's internal data structures
	 * @function
	 * @name module:contrato.Contract#rebuild
	 * @protected
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.rebuild()
	 */
	rebuild() {
		const tree = buildChildrentree(this);
		if (Object.keys(tree).length > 0) {
			this.raw.children = tree;
		}
		this.metadata.requirements = {
			matchers: {},
			types: new Set(),
			compiled: new ObjectSet(),
		};
		/**
		 * @summary Create and register a requirements matcher
		 * @function
		 * @private
		 *
		 * @param {Object} data - matcher data
		 * @returns {Object} matcher object
		 *
		 * @example
		 * const matcher = registerMatcher({
		 *   type: 'arch.sw',
		 *   slug: 'armv7hf'
		 * })
		 */
		const registerMatcher = (data: any): any => {
			const matcher = Contract.createMatcher(data);
			if (!this.metadata.requirements.matchers[data.type]) {
				this.metadata.requirements.matchers[data.type] = new ObjectSet();
			}
			this.metadata.requirements.matchers[data.type].add(matcher, {
				id: matcher.metadata.hash,
			});
			this.metadata.requirements.types.add(data.type);
			return matcher;
		};
		for (const conjunct of this.raw.requires || []) {
			if (conjunct.type) {
				const matcher = registerMatcher(conjunct);
				this.metadata.requirements.compiled.add(matcher, {
					id: matcher.metadata.hash,
				});
				continue;
			}
			const operand = first(keys(conjunct));
			if (operand) {
				const matchers = new ObjectSet();
				for (const disjunct of conjunct[operand]) {
					const matcher = registerMatcher(disjunct);
					matchers.add(matcher, {
						id: matcher.metadata.hash,
					});
				}
				const operationContract = Contract.createMatcher(matchers, {
					operation: operand,
				});
				this.metadata.requirements.compiled.add(operationContract, {
					id: operationContract.metadata.hash,
				});
			}
		}
	}
	/**
	 * @summary Interpolate the contract's template
	 * @function
	 * @name module:contrato.Contract#interpolate
	 * @protected
	 *
	 * @param {Object} [options] - options
	 * @param {Boolean} [options.rehash=true] - whether to re-hash the contract
	 * @returns {Object} contract instance
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.interpolate()
	 */
	interpolate(options: object = { rehash: Boolean }): this {
		// TODO: Find a way to keep track of whether the contract
		// has already been fully templated, and if so, avoid
		// running this function.
		this.raw = compileContract(this.raw, {
			// Each contract is only templated using its own
			// properties, so here we prevent interpolations
			// on children using the master contract as a root.
			blacklist: new Set(['children']),
		});
		defaults(options, {
			rehash: true,
		});
		this.rebuild();
		if ((options as any).rehash) {
			this.hash();
		}
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
	getVersion(): string {
		return this.raw.version;
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
	getSlug(): string {
		return this.raw.slug;
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
		const slugs = new Set<string>(this.raw.aliases);
		slugs.add(this.getSlug());
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
		return Boolean(this.raw.aliases) && this.raw.aliases.length > 0;
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
	getCanonicalSlug(): string {
		return this.raw.canonicalSlug || this.getSlug();
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
		return this.raw.type;
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
		const slug = this.getSlug();
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
		return Object.assign({}, this.raw);
	}
	/**
	 * @summary Add a child contract
	 * @function
	 * @name module:contrato.Contract#addChild
	 * @public
	 *
	 * @param {Object} contract - contract
	 * @param {Object} [options] - options
	 * @param {Boolean} [options.rehash=true] - whether to re-hash the parent contract
	 * @param {Boolean} [options.rebuild=true] - whether to re-build the parent contract
	 * @returns {Object} contract
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChild(new Contract({ ... }))
	 */
	addChild(contract: Contract, options: object = {}): this {
		const type = contract.getType();
		if (this.metadata.children.map[contract.metadata.hash]) {
			return this;
		}
		if (!this.metadata.children.types.has(type)) {
			this.metadata.children.types.add(type);
			this.metadata.children.byType[type] = new Set();
			this.metadata.children.byTypeSlug[type] = {};
		}
		for (const slug of contract.getAllSlugs()) {
			if (!this.metadata.children.byTypeSlug[type][slug]) {
				this.metadata.children.byTypeSlug[type][slug] = new Set();
			}
			this.metadata.children.byTypeSlug[type][slug].add(contract.metadata.hash);
		}
		this.metadata.children.map[contract.metadata.hash] = contract;
		this.metadata.children.byType[type].add(contract.metadata.hash);
		this.metadata.children.searchCache.resetType(type);
		defaults(options, {
			rehash: true,
			rebuild: true,
		});
		if ((options as any).rebuild) {
			this.rebuild();
		}
		if ((options as any).rehash) {
			this.hash();
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
	 * @param {Object} [options] - options
	 * @param {Boolean} [options.rehash=true] - whether to rehash the contract
	 * @returns {Object} parent contract
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 *
	 * const child = new Contract({ ... })
	 * contract.addChild(child)
	 * contract.removeChild(child)
	 */
	removeChild(contract: Contract, options: object = {}): this {
		defaults(options, {
			rehash: true,
		});
		const type = contract.getType();
		const childHash = contract.metadata.hash;
		if (!this.raw.children || !this.metadata.children.map[childHash]) {
			return this;
		}
		Reflect.deleteProperty(this.metadata.children.map, childHash);
		this.metadata.children.byType[type].delete(childHash);
		if (this.metadata.children.byType[type].size === 0) {
			Reflect.deleteProperty(this.metadata.children.byType, type);
			this.metadata.children.types.delete(type);
		}
		for (const slug of contract.getAllSlugs()) {
			this.metadata.children.byTypeSlug[type][slug].delete(childHash);
			if (this.metadata.children.byTypeSlug[type][slug].size === 0) {
				Reflect.deleteProperty(this.metadata.children.byTypeSlug[type], slug);
			}
		}
		if (Object.keys(this.metadata.children.byTypeSlug[type]).length === 0) {
			Reflect.deleteProperty(this.metadata.children.byTypeSlug, type);
		}
		this.metadata.children.searchCache.resetType(contract.getType());
		this.rebuild();
		if ((options as any).rehash) {
			this.hash();
		}
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
	 * @param {Object} [options] - options
	 * @param {Boolean} [options.rehash=true] - whether to rehash the contract
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
	addChildren(contracts: Contract[] = [], options: object = {}): this {
		if (!contracts) {
			return this;
		}
		defaults(options, {
			rehash: true,
		});
		for (const contract of contracts) {
			this.addChild(contract, {
				// For performance reasons. If this is set to true,
				// then we would re-build the contract N times, where
				// N is the number of contracts passed to this function.
				// Intead, we can prevent re-building and only do it
				// once when the function completes.
				rehash: false,
				rebuild: false,
			});
		}
		this.rebuild();
		if ((options as any).rehash) {
			this.hash();
		}
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
		const types = new Set<string>(this.metadata.children.types);
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
		return this.metadata.children.map[childHash];
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
	getChildren(options: object = {}): Contract[] {
		const contracts: Contract[] = [];
		forEach(this.metadata.children.map, (contract) => {
			if (
				!(options as any).types ||
				(options as any).types.has(contract.raw.type)
			) {
				contracts.push(contract);
			}
			contracts.push(...contract.getChildren(options));
		});
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
		if (!this.metadata.children.typeMatchers[type]) {
			this.metadata.children.typeMatchers[type] = Contract.createMatcher({
				type,
			});
		}
		return this.findChildren(this.metadata.children.typeMatchers[type]);
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
		if (!matcher.raw) {
			return [];
		}
		const results: Contract[] = [];
		for (const contract of this.getChildren().concat([this])) {
			// We need to omit the slug from the matcher object, otherwise
			// matchers that use an alias as a slug will never match the
			// structure of the actual contract.
			// Notice we do use the slug key separately, in order to obtain
			// the list of hashes we should check against.
			const match = matches(omit(matcher.raw.data, ['slug', 'version']));
			const versionMatch = matcher.raw.data.version;
			if (contract.raw.capabilities) {
				for (const capability of contract.raw.capabilities) {
					if (match(capability)) {
						if (versionMatch) {
							if (valid(capability.version) && validRange(versionMatch)) {
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
	findChildren(matcher: Contract | object): Contract[] {
		if (
			!(matcher instanceof Contract) ||
			!matcher.raw ||
			!this.getChildrenTypes().has(matcher.raw.data.type)
		) {
			return [];
		}
		const cache = this.metadata.children.searchCache.get(matcher);
		if (cache) {
			return cache;
		}
		const results: Contract[] = [];
		const type = matcher.raw.data.type;
		const slug = matcher.raw.data.slug;
		for (const contract of this.getChildren().concat([this])) {
			if (!contract.metadata.children.types.has(type)) {
				continue;
			}
			// We need to omit the slug from the matcher object, otherwise
			// matchers that use an alias as a slug will never match the
			// structure of the actual contract.
			// Notice we do use the slug key separately, in order to obtain
			// the list of hashes we should check against.
			const match = matches(omit(matcher.raw.data, ['slug', 'version']));
			const versionMatch = matcher.raw.data.version;
			const hashes = slug
				? contract.metadata.children.byTypeSlug[type][slug] || new Set()
				: contract.metadata.children.byType[type];
			// Means that we are matching just the type
			if (Object.keys(matcher.raw.data).length === 1) {
				for (const childHash of hashes) {
					const child = contract.getChildByHash(childHash);
					if (!child) {
						throw new Error('Error retreiving child');
					}
					results.push(child);
				}
			} else {
				for (const childHash of hashes) {
					const child = contract.getChildByHash(childHash);
					if (child && match(child.raw)) {
						if (versionMatch) {
							if (valid(child.raw.version) && validRange(versionMatch)) {
								if (satisfies(child.raw.version, versionMatch)) {
									results.push(child);
								}
							} else if (isEqual(child.raw.version, versionMatch)) {
								results.push(child);
							}
							continue;
						}
						results.push(child);
					}
				}
			}
		}
		this.metadata.children.searchCache.add(matcher, results);
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
		from: number;
		to: number;
		[index: string]: any;
	}): Contract[][] {
		let contracts = this.getChildrenByType(options.type);
		const cardinality = options['cardinality'] || options;
		if (options['filter']) {
			const filterValidator = partial(isValid, options['filter']);
			contracts = filter(contracts, (con) => {
				return filterValidator(con.raw);
			});
		}
		if (contracts.length > 0) {
			if (options['version']) {
				if (isEqual(options['version'], 'latest')) {
					contracts.sort((left, right) => {
						return compare(right.raw.version, left.raw.version);
					});
					contracts = contracts.slice(
						0,
						Math.min(contracts.length, cardinality.to),
					);
				} else {
					contracts = filter(contracts, (con) => {
						return satisfies(con.raw.version, options['version']);
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
		return flatMap(rang, (tcardinality) => {
			return bigCombination(contracts, tcardinality).toArray();
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
			if (!this.metadata.requirements.types.has(type)) {
				continue;
			}
			references[type] = [];
			const matchers = this.metadata.requirements.matchers[type].getAll();
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
			(accumulator, value: Contract[][]) => {
				return accumulator.concat(intersectionWith(...value, Contract.isEqual));
			},
			[] as Contract[],
		);
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
	getNotSatisfiedChildRequirements(
		contract: Contract,
		options: { types: Set<string> } = { types: new Set() },
	): any[] {
		const conjuncts = reduce(
			contract.getChildren(),
			(accumulator, child) => {
				return accumulator.concat(
					child.metadata.requirements.compiled.getAll(),
				);
			},
			contract.metadata.requirements.compiled.getAll(),
		);
		// (1) If the top level list of conjuncts is empty,
		// then we can assume the requirements are fulfilled
		// and stop without doing any further computations.
		if (conjuncts.length === 0) {
			return [];
		}
		// Utilities
		const shouldEvaluateType = (type: string) =>
			options.types ? options.types.has(type) : true;

		const requirements: any[] = [];
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
		// (2) The requirements are specified as a list of objects,
		// so lets iterate through those.
		// This function uses a for loop instead of a more functional
		// construct for performance reasons, given that we can freely
		// break out of the loop as soon as possible.
		for (const conjunct of conjuncts) {
			if (conjunct.raw.operation === 'or') {
				// (3.1) Note that we should only consider disjuncts
				// of types we are allowed to check. We can make
				// such transformation here, so we can then consider
				// the disjunction as fulfilled if there are no
				// remaining disjuncts.
				const disjuncts = filter(conjunct.raw.data.getAll(), (disjunct) => {
					return shouldEvaluateType(disjunct.raw.data.type);
				});
				// (3.2) An empty disjuction means that this particular
				// requirement is fulfilled, so we can carry on.
				// A disjunction naturally contains a list of further
				// requirements we need to check for. If at least one
				// of the members is fulfilled, we can proceed with
				// next requirement.
				if (disjuncts.length === 0 || some(map(disjuncts, hasMatch))) {
					continue;
				}
				// (3.3) If no members were fulfilled, then we know
				// that this requirement was not fullfilled, so it will be returned
				requirements.push(conjunct.raw.data);
			}
			// (4) If we should evaluate this requirement and it is not fullfilled
			// it will be returned
			if (shouldEvaluateType(conjunct.raw.data.type) && !hasMatch(conjunct)) {
				requirements.push(conjunct.raw.data);
			} else if (!shouldEvaluateType(conjunct.raw.data.type)) {
				// If this requirement is not evaluated, because of missing contracts,
				// it will also be returned.
				requirements.push(conjunct.raw.data);
			}
		}
		// (5) If we reached this far, then it means that all the
		// requirements were checked, and they were all satisfied,
		// so this is good to go!
		return requirements;
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
		const conjuncts = reduce(
			contract.getChildren(),
			(accumulator, child) => {
				return accumulator.concat(
					child.metadata.requirements.compiled.getAll(),
				);
			},
			contract.metadata.requirements.compiled.getAll(),
		);
		// (1) If the top level list of conjuncts is empty,
		// then we can assume the requirements are fulfilled
		// and stop without doing any further computations.
		if (conjuncts.length === 0) {
			return true;
		}
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
		const hasMatch = (matcher: Contract): boolean =>
			// TODO: Write a function similar to findContracts
			// that stops as soon as it finds one match
			this.findChildren(matcher).length > 0;
		// (2) The requirements are specified as a list of objects,
		// so lets iterate through those.
		// This function uses a for loop instead of a more functional
		// construct for performance reasons, given that we can freely
		// break out of the loop as soon as possible.
		for (const conjunct of conjuncts) {
			if (conjunct.raw.operation === 'or') {
				// (3.1) Note that we should only consider disjuncts
				// of types we are allowed to check. We can make
				// such transformation here, so we can then consider
				// the disjunction as fulfilled if there are no
				// remaining disjuncts.
				const disjuncts = filter(conjunct.raw.data.getAll(), (disjunct) => {
					return shouldEvaluateType(disjunct.raw.data.type);
				});
				// (3.2) An empty disjuction means that this particular
				// requirement is fulfilled, so we can carry on.
				// A disjunction naturally contains a list of further
				// requirements we need to check for. If at least one
				// of the members is fulfilled, we can proceed with
				// next requirement.
				if (disjuncts.length === 0 || some(map(disjuncts, hasMatch))) {
					continue;
				}
				// (3.3) If no members were fulfilled, then we know
				// the whole contract is unsatisfied, so there's no
				// reason to keep checking the remaining requirements.
				return false;
			} else if (conjunct.raw.operation === 'not') {
				// (3.4) Note that we should only consider disjuncts
				// of types we are allowed to check. We can make
				// such transformation here, so we can then consider
				// the disjunction as fulfilled if there are no
				// remaining disjuncts.
				const disjuncts = filter(conjunct.raw.data.getAll(), (disjunct) => {
					return shouldEvaluateType(disjunct.raw.data.type);
				});
				// (3.5) We fail the requirement if the set of negated
				// disjuncts is not empty, and we have at least one of
				// them in the context.
				if (disjuncts.length > 0 && some(map(disjuncts, hasMatch))) {
					return false;
				}
				continue;
			}
			// (4) If we reached this point, then we know we're dealing
			// with a conjunct from the top level *AND* operator.
			// Since a logical "and" means that all elements must be
			// fulfilled, we can return right away if one of these
			// was not satisfied.
			if (shouldEvaluateType(conjunct.raw.data.type) && !hasMatch(conjunct)) {
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
				areSetsDisjoint(options.types, contract.metadata.requirements.types)
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
	getAllNotSatisfiedChildRequirements(options: object = {}): any[] {
		let requirements: any[] = [];
		for (const contract of this.getChildren()) {
			// The contract object keeps track of which contract
			// types the contract references in the requirements.
			// If we specified a set of types and we know this
			// contract is not interested in them, then we can
			// continue and avoid traversing through all the
			// requirements in vain.
			if (
				(options as any).types &&
				areSetsDisjoint(
					(options as any).types,
					contract.metadata.requirements.types,
				)
			) {
				requirements = concat(
					requirements,
					map(contract.metadata.requirements.compiled.getAll(), 'data'),
				);
				continue;
			}
			const contractRequirements = this.getNotSatisfiedChildRequirements(
				contract,
				{
					types: (options as any).types,
				},
			);
			requirements = concat(requirements, contractRequirements);
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
	 * @param {(Object|Object[])} data - matcher data
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
		data: object | object[],
		options: { operation?: string } = {},
	): Contract {
		return new Contract({
			type: MATCHER,
			operation: options.operation,
			data,
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
		if (contract1.metadata.hash && contract2.metadata.hash) {
			return contract1.metadata.hash === contract2.metadata.hash;
		}
		return isEqual(contract1.raw, contract2.raw);
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
		return reduce(
			rawContracts,
			(accumulator, variant) => {
				const aliases = variant['aliases'] || [];
				const obj = omit(variant, ['aliases']) as ContractObject;
				const contracts = map(aliases, (alias) => {
					return new Contract(
						Object.assign({}, obj, {
							canonicalSlug: obj['slug'],
							slug: alias,
						}),
					);
				});
				contracts.push(new Contract(obj));
				return accumulator.concat(contracts);
			},
			[] as Contract[],
		);
	}
}
