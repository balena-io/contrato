/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';
import CONTRACTS from '../contracts.json';

describe('Contract getChilredCombinations', () => {
	it('should throw if the type is not valid', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
		]);

		expect(() => {
			container.getChildrenCombinations({
				type: 'foo',
				from: 2,
				to: 2,
			});
		}).throw(
			'Invalid cardinality: 2 to 2. The number of foo contracts in the universe is 0',
		);
	});

	it('should return combinations of cardinality 1 for one contract', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
		]);

		expect(
			container.getChildrenCombinations({
				type: 'sw.os',
				from: 1,
				to: 1,
			}),
		).to.equalByHash([[new Contract(CONTRACTS['sw.os'].debian.wheezy.object)]]);
	});

	it('should return combinations of cardinality 1 for two contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
		]);

		expect(
			container.getChildrenCombinations({
				type: 'sw.os',
				from: 1,
				to: 1,
			}),
		).to.equalByHash([
			[new Contract(CONTRACTS['sw.os'].debian.wheezy.object)],
			[new Contract(CONTRACTS['sw.os'].debian.jessie.object)],
		]);
	});

	it('should return combinations of cardinality 1 for three contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['sw.os'].fedora['25'].object),
		]);

		expect(
			container.getChildrenCombinations({
				type: 'sw.os',
				from: 1,
				to: 1,
			}),
		).to.equalByHash([
			[new Contract(CONTRACTS['sw.os'].debian.wheezy.object)],
			[new Contract(CONTRACTS['sw.os'].debian.jessie.object)],
			[new Contract(CONTRACTS['sw.os'].fedora['25'].object)],
		]);
	});

	it('should return combinations of cardinality 2 for two contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
		]);

		expect(
			container.getChildrenCombinations({
				type: 'sw.os',
				from: 2,
				to: 2,
			}),
		).to.equalByHash([
			[
				new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
				new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			],
		]);
	});

	it('should return combinations of cardinality 2 for three contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['sw.os'].fedora['25'].object),
		]);

		expect(
			container.getChildrenCombinations({
				type: 'sw.os',
				from: 2,
				to: 2,
			}),
		).to.equalByHash([
			[
				new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
				new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			],
			[
				new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
				new Contract(CONTRACTS['sw.os'].fedora['25'].object),
			],
			[
				new Contract(CONTRACTS['sw.os'].debian.jessie.object),
				new Contract(CONTRACTS['sw.os'].fedora['25'].object),
			],
		]);
	});

	it('should throw if "from" is greater than "to"', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
		]);

		expect(() => {
			container.getChildrenCombinations({
				type: 'sw.os',
				from: 2,
				to: 1,
			});
		}).throw(
			'Invalid cardinality: 2 to 1. The starting point is greater than the ending point',
		);
	});

	it('should generate combinations from 1 to 2 for one contract', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
		]);

		expect(
			container.getChildrenCombinations({
				type: 'sw.os',
				from: 1,
				to: 2,
			}),
		).to.equalByHash([[new Contract(CONTRACTS['sw.os'].debian.wheezy.object)]]);
	});

	it('should return combinations from 1 to 2 for two contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
		]);

		expect(
			container.getChildrenCombinations({
				type: 'sw.os',
				from: 1,
				to: 2,
			}),
		).to.equalByHash([
			[new Contract(CONTRACTS['sw.os'].debian.wheezy.object)],
			[new Contract(CONTRACTS['sw.os'].debian.jessie.object)],
			[
				new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
				new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			],
		]);
	});

	it('should return combinations from 1 to 3 for two contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
		]);

		expect(
			container.getChildrenCombinations({
				type: 'sw.os',
				from: 1,
				to: 3,
			}),
		).to.equalByHash([
			[new Contract(CONTRACTS['sw.os'].debian.wheezy.object)],
			[new Contract(CONTRACTS['sw.os'].debian.jessie.object)],
			[
				new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
				new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			],
		]);
	});

	it('should return combinations from 1 to 3 for three contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.os'].fedora['25'].object),
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
		]);

		expect(
			container.getChildrenCombinations({
				type: 'sw.os',
				from: 1,
				to: 3,
			}),
		).to.equalByHash([
			[new Contract(CONTRACTS['sw.os'].fedora['25'].object)],
			[new Contract(CONTRACTS['sw.os'].debian.wheezy.object)],
			[new Contract(CONTRACTS['sw.os'].debian.jessie.object)],
			[
				new Contract(CONTRACTS['sw.os'].fedora['25'].object),
				new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			],
			[
				new Contract(CONTRACTS['sw.os'].fedora['25'].object),
				new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			],
			[
				new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
				new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			],
			[
				new Contract(CONTRACTS['sw.os'].fedora['25'].object),
				new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
				new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			],
		]);
	});

	it('should not consider aliases as separate contracts', () => {
		const contract1 = new Contract({
			type: 'hw.device-type',
			name: 'Raspberry Pi 2',
			slug: 'raspberrypi2',
			aliases: ['rpi2', 'raspberry-pi2'],
		});

		const contract2 = new Contract({
			type: 'hw.device-type',
			name: 'Raspberry Pi',
			slug: 'raspberrypi',
			aliases: ['rpi', 'raspberry-pi'],
		});

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(
			container.getChildrenCombinations({
				type: 'hw.device-type',
				from: 1,
				to: 1,
			}),
		).to.equalByHash([[contract1], [contract2]]);
	});
});
