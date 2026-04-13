import intersectionWith from 'lodash/intersectionWith';
import range from 'lodash/range';
import reduce from 'lodash/reduce';
import { Combination } from 'js-combinatorics';
import { compare, satisfies } from 'semver';

import { Contract as WasmContract } from '../contrato-wasm/pkg/contrato_wasm.js';
import { isValid } from './json-schema';
import type { ContractObject, MatcherObject } from './types';

function typesArg(types?: Set<string>): string[] | undefined {
	return types ? [...types] : undefined;
}

export default class Contract {
	// The WASM-backed contract. A native private field, so the opaque handle
	// cannot leak into `deep.equal` comparisons or JSON serialization.
	#inner: WasmContract;

	// `$raw` is installed as an own, *enumerable* accessor by the constructor
	// so that structural equality (chai `deep.equal`) compares two contracts
	// by their JSON content — including children — rather than by the opaque
	// WASM handle.
	declare protected $raw: ContractObject;

	/**
	 * @summary Get a deep copy of the raw serializable contract
	 * @function
	 * @name module:contrato.Contract#raw
	 * @public
	 *
	 * @returns {ContractObject} the JSON content of the contract
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * console.log(contract.raw())
	 */
	public raw(): ContractObject {
		// `$raw` crosses the WASM boundary and yields a fresh object on every
		// access, so no further cloning is needed to keep callers from mutating
		// the contract's internals.
		return this.$raw;
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
		// An existing WASM handle is adopted as-is (see `fromWasm`). The
		// parameter stays typed as `ContractObject` so a handle can never be
		// passed in from outside this module.
		this.#inner =
			object instanceof WasmContract ? object : new WasmContract(object);
		Object.defineProperty(this, '$raw', {
			get: (): ContractObject => this.#inner.toJSON(),
			enumerable: true,
			configurable: true,
		});
	}

	// Wraps a handle returned by WASM.
	protected static fromWasm(inner: WasmContract): Contract {
		return new Contract(inner as unknown as ContractObject);
	}

	/**
	 * @summary Get the contract hash, computing it lazily if necessary
	 * @function
	 * @name module:contrato.Contract#hash
	 * @protected
	 *
	 * @description
	 * The hash is computed from the contract's raw object the first time
	 * it is requested, and cached on the Rust side afterwards. Operations
	 * that mutate the contract invalidate that cache, so the hash is
	 * recomputed on the next call.
	 *
	 * @returns {String} the contract hash
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * console.log(contract.hash())
	 */
	hash(): string {
		return this.#inner.hash();
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
		this.#inner.interpolate();
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
		return this.#inner.getVersion();
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
		return this.#inner.getSlug();
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
		return new Set(this.#inner.getAllSlugs() as string[]);
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
		return this.#inner.hasAliases();
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
		return this.#inner.getCanonicalSlug() ?? this.getSlug();
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
		return this.#inner.getType();
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
		return this.#inner.getReferenceString();
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
		return this.$raw;
	}

	/**
	 * @summary Add a child contract
	 * @function
	 * @name module:contrato.Contract#addChild
	 * @public
	 *
	 * @param {Object} contract - contract
	 * @returns {Object} contract
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChild(new Contract({ ... }))
	 */
	addChild(contract: Contract): this {
		this.#inner.addChild(contract.#inner);
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
		this.#inner.removeChild(contract.#inner);
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
		// we clone the passed contracts because the Rust side consumes the array
		this.#inner.addChildren(contracts.map((c) => new WasmContract(c.$raw)));
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
		return new Set(this.#inner.getChildrenTypes() as string[]);
	}

	/**
	 * @summary Get a single child by its hash
	 * @function
	 * @name module:contrato.Contract#getChildByHash
	 * @public
	 *
	 * @param {String} hash - child contract hash
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
	getChildByHash(hash: string): Contract | undefined {
		const child = this.#inner.getChildByHash(hash);
		if (!child) {
			return undefined;
		}
		return Contract.fromWasm(child);
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
		if (options.types) {
			if (options.types.size === 0) {
				return [];
			}
			return (
				this.#inner.getChildrenByTypes([...options.types]) as WasmContract[]
			).map((c) => Contract.fromWasm(c));
		}
		return (this.#inner.getChildren() as WasmContract[]).map((c) =>
			Contract.fromWasm(c),
		);
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
		return (this.#inner.getChildrenByType(type) as WasmContract[]).map((c) =>
			Contract.fromWasm(c),
		);
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
	findChildren(matcher: MatcherObject): Contract[] {
		if (!matcher || !('type' in matcher) || !matcher.type) {
			return [];
		}
		return (this.#inner.findChildren(matcher) as WasmContract[]).map((c) =>
			Contract.fromWasm(c),
		);
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
		const cardinality = options['cardinality'] ?? options;
		if (options['filter']) {
			contracts = contracts.filter((con) => {
				return isValid(options['filter'], con.$raw);
			});
		}
		if (contracts.length > 0) {
			if (options['version']) {
				if (options['version'] === 'latest') {
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
		// XXX: this function crosses the WASM boundary O(N×M×K) times (N contracts × M
		// requirement types × K matchers per type). Each iteration serializes
		// matchers out, calls findChildren, then recurses. A Rust
		// implementation would operate on &Contract references, use the cached
		// search path, and return the final map in a single boundary crossing.
		const references: { [index: string]: Contract[] } = {};
		const reqTypes = this.#inner.getRequirementTypes() as string[];
		for (const type of options.types) {
			if (!reqTypes.includes(type)) {
				continue;
			}
			references[type] = [];
			const matchers = this.#inner.getRequirementMatchersForType(
				type,
			) as MatcherObject[];
			for (const matcher of matchers) {
				for (const find of options.from.findChildren(matcher)) {
					references[find.getType()] ??= [];
					references[find.getType()].push(find);
					const nested = find.getReferencedContracts(options);
					for (const nestedType of Object.keys(nested)) {
						references[nestedType] ??= [];
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
		// FIXME: Compounds the boundary-crossing cost of getReferencedContracts
		// by calling it once per child, then intersects in JS with lodash.
		// Moving to Rust would collapse the entire walk + intersection into a
		// single WASM call using hash-based set intersection.

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
				return accumulator.concat(
					intersectionWith(
						...(value as [Contract[], ...Contract[][]]),
						Contract.isEqual,
					),
				);
			},
			[] as Contract[],
		);
	}

	/**
	 * @summary Get a list of child requirements that are not satisfied by this contract
	 * @function
	 * @name module:contrato.Contract#getNotSatisfiedChildRequirements
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
	 * console.log(contract.getNotSatisfiedChildRequirements(child))
	 */
	getNotSatisfiedChildRequirements(
		contract: Contract,
		options: { types?: Set<string> } = {},
	): any[] {
		return this.#inner.getNotSatisfiedChildRequirements(
			contract.#inner,
			typesArg(options.types),
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
	satisfiesChildContract(
		contract: Contract,
		options: { types?: Set<string> } = {},
	): boolean {
		return this.#inner.satisfiesChildContract(
			contract.#inner,
			typesArg(options.types),
		);
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
		return this.#inner.areChildrenSatisfied(typesArg(options.types));
	}

	/**
	 * @summary Get a list of all the child requirements that are not satisfied
	 * @function
	 * @name module:contrato.Contract#getAllNotSatisfiedChildRequirements
	 * @public
	 *
	 * @param {Object} [options] - options
	 * @param {Set} [options.types] - the types to consider (all by default)
	 * @returns list of unsatisfied requirements
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChildren([ ... ])
	 *
	 * console.log(contract.getAllNotSatisfiedChildRequirements({
	 *   types: new Set([ 'sw.arch' ])
	 * }))
	 */
	getAllNotSatisfiedChildRequirements(
		options: { types?: Set<string> } = {},
	): any[] {
		return this.#inner.getAllNotSatisfiedChildRequirements(
			typesArg(options.types),
		);
	}

	/**
	 * @summary Create a contract matcher
	 * @function
	 * @static
	 * @name module:contrato.Contract.createMatcher
	 * @public
	 *
	 * @description
	 * A matcher allows to search for child contracts by type, slug, version
	 * range and data. It is a plain object handed straight to the WASM
	 * boundary, where `contrato::ContractMatcher` validates it — matchers
	 * carrying fields other than `type`, `slug`, `version` and `data` are
	 * rejected there, at `findChildren` time.
	 *
	 * @param {Object} obj - the match criteria
	 * @returns {MatcherObject} matcher
	 *
	 * @example
	 * // find all child contracts with type `hw.device-type` and `data`
	 * // containing `{arch: 'armv7hf'}`
	 * mycontract.findChildren(Contract.createMatcher({
	 *   type: 'hw.device-type',
	 *   data: { arch: 'armv7hf' },
	 * }));
	 */
	static createMatcher(obj: MatcherObject): MatcherObject {
		return obj;
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
		return WasmContract.isEqual(contract1.#inner, contract2.#inner);
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
		return (WasmContract.build(source) as WasmContract[]).map((c) =>
			Contract.fromWasm(c),
		);
	}

	/**
	 * @summary Return an independent copy of the contract with its own WASM handle
	 * @function
	 * @name module:contrato.Contract#toJSON
	 * @public
	 *
	 * @returns {Contract} - the contract clone
	 *
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * const clone = contract.clone()
	 */
	clone(): Contract {
		// Returns an independent copy backed by its own WASM handle. Needed
		// because the internal `#inner` handle is a native private field, so a
		// generic (shallow) clone would drop it and leave a broken contract.
		return new Contract(this.$raw);
	}
}
