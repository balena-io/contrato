/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';
import CONTRACTS from '../contracts.json';

describe('Contract getChildren', () => {
	it('should get nothing if no children', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		expect(container.getChildren()).to.deep.equal([]);
	});

	it('should get the paths of a one child container', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);

		expect(container.getChildren().map((c) => c.hash())).to.deep.equal([
			contract1.hash(),
		]);
	});

	it('should get the paths of two contracts with different slugs', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(
			container
				.getChildren()
				.map((c) => c.hash())
				.sort(),
		).to.deep.equal([contract1.hash(), contract2.hash()].sort());
	});

	it('should get the paths of two contracts with same slugs', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(
			container
				.getChildren()
				.map((c) => c.hash())
				.sort(),
		).to.deep.equal([contract1.hash(), contract2.hash()].sort());
	});

	it('should be able to filter by one type', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(
			container
				.getChildren({
					types: new Set(['sw.os']),
				})
				.map((c) => c.hash()),
		).to.deep.equal([contract1.hash()]);
	});

	it('should be able to filter by two types', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		const contract3 = new Contract(CONTRACTS['hw.device-type'].artik10.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2, contract3]);

		expect(
			container
				.getChildren({
					types: new Set(['sw.os', 'sw.blob']),
				})
				.map((c) => c.hash())
				.sort(),
		).to.deep.equal([contract1.hash(), contract2.hash()].sort());
	});

	it('should ignore unknown types', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(
			container
				.getChildren({
					types: new Set(['sw.os', 'hello']),
				})
				.map((c) => c.hash()),
		).to.deep.equal([contract1.hash()]);
	});

	it('should return an empty array if no type matches', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(
			container.getChildren({
				types: new Set(['hello', 'world']),
			}),
		).to.deep.equal([]);
	});

	it('should not return the same contract multiple times if it contains aliases', () => {
		const contract1 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
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
			container
				.getChildren()
				.map((c) => c.hash())
				.sort(),
		).to.deep.equal([contract1.hash(), contract2.hash()].sort());
	});

	it('should return nested children', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		contract1.addChild(contract2);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1]);

		expect(
			container
				.getChildren()
				.map((c) => c.hash())
				.sort(),
		).to.deep.equal([contract1.hash(), contract2.hash()].sort());
	});

	it('should return two level nested children', () => {
		const contract1 = new Contract(CONTRACTS['hw.device-type'].artik10.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract3 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		contract2.addChild(contract3);
		contract1.addChild(contract2);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1]);

		expect(
			container
				.getChildren()
				.map((c) => c.hash())
				.sort(),
		).to.deep.equal(
			[contract1.hash(), contract2.hash(), contract3.hash()].sort(),
		);
	});

	it('should return nested children that match the desired type', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		contract1.addChild(contract2);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1]);

		expect(
			container
				.getChildren({
					types: new Set(['sw.blob']),
				})
				.map((c) => c.hash()),
		).to.deep.equal([contract2.hash()]);
	});
});
