import intersectionWith from 'lodash/intersectionWith';
import range from 'lodash/range';
import reduce from 'lodash/reduce';
import { Combination } from 'js-combinatorics';
import { compare, satisfies } from 'semver';

import { Contract as WasmContract } from '../contrato-wasm/pkg/contrato_wasm.js';
import { isValid } from './json-schema';
import type { ContractObject } from './types';

export interface ContractMatcher {
	type: string;
	slug?: string;
	version?: string;
	data?: Record<string, any>;
}

function typesArg(types?: Set<string>): string[] | undefined {
	return types ? [...types] : undefined;
}

export default class Contract {
	protected inner: WasmContract;
	private _hash: string | undefined;

	constructor(object: ContractObject) {
		this.inner = new WasmContract(object);
	}

	protected static fromWasm(inner: WasmContract): Contract {
		const instance = Object.create(Contract.prototype) as Contract;
		instance.inner = inner;
		return instance;
	}

	getType(): string {
		return this.inner.getType();
	}

	getSlug(): string | undefined {
		return this.inner.getSlug();
	}

	getVersion(): string | undefined {
		return this.inner.getVersion();
	}

	getCanonicalSlug(): string | undefined {
		return this.inner.getCanonicalSlug() ?? this.getSlug();
	}

	getReferenceString(): string {
		return this.inner.getReferenceString();
	}

	getAllSlugs(): Set<string> {
		return new Set(this.inner.getAllSlugs() as string[]);
	}

	hasAliases(): boolean {
		return this.inner.hasAliases();
	}

	hash(): string {
		this._hash ??= this.inner.hash();
		return this._hash;
	}

	get raw(): ContractObject {
		return this.inner.toJSON();
	}

	toJSON(): ContractObject {
		return this.raw;
	}

	interpolate(): this {
		this.inner.interpolate();
		this._hash = undefined;
		return this;
	}

	addChild(contract: Contract): this {
		this.inner.addChild(contract.inner);
		this._hash = undefined;
		return this;
	}

	addChildren(contracts: Contract[] = []): this {
		if (contracts.length === 0) {
			return this;
		}
		for (const contract of contracts) {
			this.inner.addChild(contract.inner);
		}
		this._hash = undefined;
		return this;
	}

	removeChild(contract: Contract): this {
		this.inner.removeChild(contract.inner);
		this._hash = undefined;
		return this;
	}

	getChildByHash(hash: string): Contract | undefined {
		const child = this.inner.getChildByHash(hash);
		if (!child) {
			return undefined;
		}
		return Contract.fromWasm(child);
	}

	getChildren(options: { types?: Set<string> } = {}): Contract[] {
		if (options.types) {
			if (options.types.size === 0) {
				return [];
			}
			return (
				this.inner.getChildrenByTypes([...options.types]) as WasmContract[]
			).map((c) => Contract.fromWasm(c));
		}
		return (this.inner.getChildren() as WasmContract[]).map((c) =>
			Contract.fromWasm(c),
		);
	}

	getChildrenByType(type: string): Contract[] {
		return (this.inner.getChildrenByType(type) as WasmContract[]).map((c) =>
			Contract.fromWasm(c),
		);
	}

	getChildrenTypes(): Set<string> {
		return new Set(this.inner.getChildrenTypes() as string[]);
	}

	findChildren(matcher: ContractMatcher): Contract[] {
		if (!matcher || !('type' in matcher) || !matcher.type) {
			return [];
		}
		return (this.inner.findChildren(matcher) as WasmContract[]).map((c) =>
			Contract.fromWasm(c),
		);
	}

	satisfiesChildContract(
		contract: Contract,
		options: { types?: Set<string> } = {},
	): boolean {
		return this.inner.satisfiesChildContract(
			contract.inner,
			typesArg(options.types),
		);
	}

	getNotSatisfiedChildRequirements(
		contract: Contract,
		options: { types?: Set<string> } = {},
	): any[] {
		return this.inner.getNotSatisfiedChildRequirements(
			contract.inner,
			typesArg(options.types),
		);
	}

	areChildrenSatisfied(options: { types?: Set<string> } = {}): boolean {
		return this.inner.areChildrenSatisfied(typesArg(options.types));
	}

	getAllNotSatisfiedChildRequirements(
		options: { types?: Set<string> } = {},
	): any[] {
		return this.inner.getAllNotSatisfiedChildRequirements(
			typesArg(options.types),
		);
	}

	getChildrenCombinations(options: {
		type: string;
		from: number;
		to: number;
		[index: string]: any;
	}): Contract[][] {
		let contracts = this.getChildrenByType(options.type);
		const cardinality = options['cardinality'] ?? options;
		if (options['filter']) {
			contracts = contracts.filter((con) => {
				return isValid(options['filter'], con.raw);
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

	// FIXME: Crosses the WASM boundary O(N×M×K) times (N contracts × M
	// requirement types × K matchers per type). Each iteration serializes
	// matchers out, calls findChildren, then recurses. A Rust
	// implementation would operate on &Contract references, use the cached
	// search path, and return the final map in a single boundary crossing.
	getReferencedContracts(options: { types: Set<string>; from: Contract }): {
		[index: string]: Contract[];
	} {
		const references: { [index: string]: Contract[] } = {};
		const reqTypes = this.inner.getRequirementTypes() as string[];
		for (const type of options.types) {
			if (!reqTypes.includes(type)) {
				continue;
			}
			references[type] = [];
			const matchers = this.inner.getRequirementMatchersForType(
				type,
			) as ContractMatcher[];
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

	// FIXME: Compounds the boundary-crossing cost of getReferencedContracts
	// by calling it once per child, then intersects in JS with lodash.
	// Moving to Rust would collapse the entire walk + intersection into a
	// single WASM call using hash-based set intersection.
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
		return reduce<typeof result, Contract[]>(
			result,
			(accumulator, value) => {
				return accumulator.concat(
					intersectionWith(
						...(value as [Contract[], ...Contract[][]]),
						Contract.isEqual,
					),
				);
			},
			[],
		);
	}

	/// Creates a contract matcher from a dictionary
	//
	// A matcher allows to search for child contracts by type, slug, version range
	// and data. All extra variables are assumed to be part of `data`.
	//
	// Example
	// ```ts
	// // find all child contracts with type `hw.device-type` and `data` containing
	// // `{arch: 'armv7hf'}`
	// mycontract.findChildren(Contract.createMatcher({
	//   type: 'hw.device-type',
	//   arch: 'armv7hf',
	// }));
	// ```
	//
	static createMatcher(obj: Record<string, any>): ContractMatcher {
		const { type, slug, version, data, ...rest } = obj;
		const matcher: ContractMatcher = { type };
		if (slug != null) {
			matcher.slug = slug;
		}
		if (version != null) {
			matcher.version = version;
		}

		const matcherData = { ...rest, ...data };
		if (Object.keys(matcherData).length > 0) {
			matcher.data = matcherData;
		}
		return matcher;
	}

	static isEqual(contract1: Contract, contract2: Contract): boolean {
		return WasmContract.isEqual(contract1.inner, contract2.inner);
	}

	static build(source: ContractObject): Contract[] {
		return (WasmContract.build(source) as WasmContract[]).map((c) =>
			Contract.fromWasm(c),
		);
	}
}
