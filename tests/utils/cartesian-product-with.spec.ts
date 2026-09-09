/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';
import * as _ from 'lodash';
import { cartesianProductWith } from '../../lib/utils';

describe('cartesianProductWith', () => {
	it('should perform a cartesian product of empty sets', () => {
		const product = cartesianProductWith(
			[[], [], []],
			(accumulator, element) => {
				return _.concat(accumulator, [element]);
			},
			[[]],
		);

		expect([...product]).to.deep.equal([]);
	});

	it('should perform a cartesian product of empty sets with a seeded accumulator', () => {
		const product = cartesianProductWith(
			[[], []],
			(accumulator: string[], element: string) => {
				return _.concat(accumulator, [element]);
			},
			[['seed']],
		);

		// There is nothing to combine, so the seed is not a combination
		expect([...product]).to.deep.equal([]);
	});

	it('should perform a cartesian product of a valid and an empty set', () => {
		const product = cartesianProductWith(
			[['foo'], [], []],
			(accumulator: string[], element: string) => {
				return _.concat(accumulator, [element]);
			},
			[[]],
		);

		expect([...product]).to.deep.equal([['foo']]);
	});

	it('should perform a cartesian product of no sets', () => {
		const product = cartesianProductWith(
			[],
			(accumulator: any[], element: any) => {
				return _.concat(accumulator, [element]);
			},
			[[]],
		);

		expect([...product]).to.deep.equal([]);
	});

	it('should perform a cartesian product of a one element set', () => {
		const product = cartesianProductWith(
			[['foo']],
			(accumulator: string[], element: string) => {
				return _.concat(accumulator, [element]);
			},
			[[]],
		);

		expect([...product]).to.deep.equal([['foo']]);
	});

	it('should calculate the cartesian product of two string sets', () => {
		const product = cartesianProductWith(
			[
				['hello', 'hi', 'hey'],
				['there', 'world', 'yo'],
			],
			(accumulator: string[], element: string) => {
				return _.concat(accumulator, [element]);
			},
			[[]],
		);

		expect([...product]).deep.equal([
			['hello', 'there'],
			['hello', 'world'],
			['hello', 'yo'],
			['hi', 'there'],
			['hi', 'world'],
			['hi', 'yo'],
			['hey', 'there'],
			['hey', 'world'],
			['hey', 'yo'],
		]);
	});

	it('should calculate the cartesian product of three string sets with different lengths', () => {
		const product = cartesianProductWith(
			[
				['hello', 'hi', 'hey'],
				['!', ','],
				['there', 'world', 'yo'],
			],
			(accumulator: string[], element: string) => {
				return _.concat(accumulator, [element]);
			},
			[[]],
		);

		expect([...product]).deep.equal([
			['hello', '!', 'there'],
			['hello', '!', 'world'],
			['hello', '!', 'yo'],
			['hello', ',', 'there'],
			['hello', ',', 'world'],
			['hello', ',', 'yo'],
			['hi', '!', 'there'],
			['hi', '!', 'world'],
			['hi', '!', 'yo'],
			['hi', ',', 'there'],
			['hi', ',', 'world'],
			['hi', ',', 'yo'],
			['hey', '!', 'there'],
			['hey', '!', 'world'],
			['hey', '!', 'yo'],
			['hey', ',', 'there'],
			['hey', ',', 'world'],
			['hey', ',', 'yo'],
		]);
	});

	it('should be able to discard combinations by returning undefined', () => {
		const product = cartesianProductWith(
			[
				['hello', 'hi', 'hey'],
				['there', 'world', 'yo'],
			],
			(accumulator: string[], element: string) => {
				if (!_.isEqual(accumulator, ['hello']) && element === 'world') {
					return undefined;
				}

				if (_.isEqual(accumulator, ['hello']) && element === 'yo') {
					return undefined;
				}

				if (_.isEqual(accumulator, ['hello']) && element === 'there') {
					return undefined;
				}

				return _.concat(accumulator, [element]);
			},
			[[]],
		);

		expect([...product]).deep.equal([
			['hello', 'world'],
			['hi', 'there'],
			['hi', 'yo'],
			['hey', 'there'],
			['hey', 'yo'],
		]);
	});

	it('should be able to discard combinations on a 3 sets product', () => {
		const product = cartesianProductWith(
			[
				[1, 2, 3],
				[4, 5, 6],
				[7, 8, 9],
			],
			(accumulator: number[], element: number) => {
				const combination = _.concat(accumulator, [element]);

				// Lets pretend we don't want any combination starting
				// with two odd numbers
				if (
					_.size(combination) > 1 &&
					_.every(combination, (item) => {
						return item % 2 === 1;
					})
				) {
					return undefined;
				}

				return combination;
			},
			[[]],
		);

		expect([...product]).deep.equal([
			[1, 4, 7],
			[1, 4, 8],
			[1, 4, 9],
			[1, 6, 7],
			[1, 6, 8],
			[1, 6, 9],
			[2, 4, 7],
			[2, 4, 8],
			[2, 4, 9],
			[2, 5, 7],
			[2, 5, 8],
			[2, 5, 9],
			[2, 6, 7],
			[2, 6, 8],
			[2, 6, 9],
			[3, 4, 7],
			[3, 4, 8],
			[3, 4, 9],
			[3, 6, 7],
			[3, 6, 8],
			[3, 6, 9],
		]);
	});
});
