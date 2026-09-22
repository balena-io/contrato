/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import chai from 'chai';
import chaiAsPromised from 'chai-as-promised';
import isEqualWith from 'lodash/isEqualWith';

import Contract from '../lib/contract';

declare global {
	// eslint-disable-next-line @typescript-eslint/no-namespace
	namespace Chai {
		interface Assertion {
			/**
			 * Deep equality where contracts are compared with `Contract#isEqual`,
			 * that is, by hash.
			 *
			 * Contracts are opaque handles over the WASM implementation, so
			 * structural equality on the JavaScript objects is meaningless.
			 * Arrays are traversed, which covers the lists (and lists of lists)
			 * of contracts returned by the API.
			 */
			equalByHash(expected: unknown): Assertion;

			/**
			 * Same as `equalByHash`, but ignoring the order of the contracts,
			 * for the operations that make no ordering guarantees.
			 */
			sameMembersByHash(expected: readonly Contract[]): Assertion;
		}
	}
}

const equalByHash = (actual: unknown, expected: unknown): boolean =>
	isEqualWith(actual, expected, (left: unknown, right: unknown) =>
		left instanceof Contract || right instanceof Contract
			? left instanceof Contract &&
				right instanceof Contract &&
				left.isEqual(right)
			: undefined,
	);

/** Contracts by ascending hash, so that a comparison ignores their order. */
const sortedByHash = (contracts: readonly Contract[]): Contract[] =>
	[...contracts].sort((left, right) => left.hash().localeCompare(right.hash()));

/**
 * The reportable form of a value: contracts stand for their raw object, so
 * that a failed assertion diffs contract content rather than opaque handles.
 */
const asRaw = (value: unknown): unknown => {
	if (value instanceof Contract) {
		return value.raw();
	}
	return Array.isArray(value) ? value.map(asRaw) : value;
};

// Reports the operands as raw contracts, projecting them only when the
// assertion is about to fail: chai reads them to build the error message.
const assertByHash = (
	assertion: Chai.AssertionStatic,
	ok: boolean,
	verb: string,
	actual: unknown,
	expected: unknown,
): void => {
	const failing = ok === Boolean(chai.util.flag(assertion, 'negate'));
	assertion.assert(
		ok,
		`expected #{act} to ${verb} #{exp}, comparing contracts by hash`,
		`expected #{act} not to ${verb} #{exp}, comparing contracts by hash`,
		failing ? asRaw(expected) : undefined,
		failing ? asRaw(actual) : undefined,
		failing,
	);
};

chai.use(chaiAsPromised);

chai.use((instance, utils) => {
	instance.Assertion.addMethod(
		'equalByHash',
		function (this: Chai.AssertionStatic, expected: unknown) {
			const actual = utils.flag(this, 'object');
			assertByHash(
				this,
				equalByHash(actual, expected),
				'equal',
				actual,
				expected,
			);
		},
	);

	instance.Assertion.addMethod(
		'sameMembersByHash',
		function (this: Chai.AssertionStatic, expected: readonly Contract[]) {
			const actual = utils.flag(this, 'object');
			assertByHash(
				this,
				Array.isArray(actual) &&
					equalByHash(sortedByHash(actual), sortedByHash(expected)),
				'have the same members as',
				actual,
				expected,
			);
		},
	);
});

export default chai;

export const { expect } = chai;
