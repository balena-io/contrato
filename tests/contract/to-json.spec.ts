/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';
import CONTRACTS from '../contracts.json';

describe('Contract toJSON', () => {
	it('should convert a contract into a JSON object', () => {
		const source = CONTRACTS['sw.os'].debian.wheezy.object;
		const contract = new Contract(source);
		expect(contract.toJSON()).to.deep.equal(source);
	});

	it('should handle a contract with one child', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(new Contract(CONTRACTS['sw.os'].debian.wheezy.object));

		expect(container.toJSON()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: CONTRACTS['sw.os'].debian.wheezy.object,
				},
			},
		});
	});

	it('should handle a contract with two children of the same type and slug', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
		]);

		expect(container.toJSON()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: {
						debian: [
							CONTRACTS['sw.os'].debian.wheezy.object,
							CONTRACTS['sw.os'].debian.jessie.object,
						],
					},
				},
			},
		});
	});

	it('should handle a contract with two children of the same type but different slugs', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].fedora['25'].object),
		]);

		expect(container.toJSON()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: {
						debian: CONTRACTS['sw.os'].debian.wheezy.object,
						fedora: CONTRACTS['sw.os'].fedora['25'].object,
					},
				},
			},
		});
	});

	it('should handle a contract with two children of different types', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
		]);

		expect(container.toJSON()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: CONTRACTS['sw.os'].debian.wheezy.object,
					blob: CONTRACTS['sw.blob'].nodejs['4.8.0'].object,
				},
			},
		});
	});

	it('should not expand one child with aliases', () => {
		const contract1 = new Contract({
			type: 'hw.device-type',
			name: 'Raspberry Pi',
			slug: 'raspberrypi',
			aliases: ['raspberry-pi', 'rpi'],
		});

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);

		expect(container.toJSON()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				hw: {
					'device-type': contract1.raw,
				},
			},
		});

		expect(new Contract(container.raw).toJSON()).to.deep.equal(container.raw);
	});

	it('should expand aliases in two children of the same type', () => {
		const contract1 = new Contract({
			type: 'hw.device-type',
			name: 'Raspberry Pi',
			slug: 'raspberrypi',
			aliases: ['raspberry-pi', 'rpi'],
		});

		const contract2 = new Contract({
			type: 'hw.device-type',
			name: 'Intel NUC',
			slug: 'intel-nuc',
			aliases: ['nuc'],
		});

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(container.toJSON()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				hw: {
					'device-type': {
						raspberrypi: contract1.raw,
						rpi: contract1.raw,
						'raspberry-pi': contract1.raw,
						'intel-nuc': contract2.raw,
						nuc: contract2.raw,
					},
				},
			},
		});

		expect(new Contract(container.raw).toJSON()).to.deep.equal(container.raw);
	});

	it('should correctly handle one aliased and one non aliased child of the same type', () => {
		const contract1 = new Contract({
			type: 'hw.device-type',
			name: 'Raspberry Pi',
			slug: 'raspberrypi',
			aliases: ['raspberry-pi', 'rpi'],
		});

		const contract2 = new Contract({
			type: 'hw.device-type',
			name: 'Intel NUC',
			slug: 'intel-nuc',
		});

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(container.toJSON()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				hw: {
					'device-type': {
						raspberrypi: contract1.raw,
						rpi: contract1.raw,
						'raspberry-pi': contract1.raw,
						'intel-nuc': contract2.raw,
					},
				},
			},
		});

		expect(new Contract(container.raw).toJSON()).to.deep.equal(container.raw);
	});

	it('should correctly handle one none aliased and one aliased child of the same type', () => {
		const contract1 = new Contract({
			type: 'hw.device-type',
			name: 'Intel NUC',
			slug: 'intel-nuc',
		});

		const contract2 = new Contract({
			type: 'hw.device-type',
			name: 'Raspberry Pi',
			slug: 'raspberrypi',
			aliases: ['raspberry-pi', 'rpi'],
		});

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(container.toJSON()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				hw: {
					'device-type': {
						raspberrypi: contract2.raw,
						rpi: contract2.raw,
						'raspberry-pi': contract2.raw,
						'intel-nuc': contract1.raw,
					},
				},
			},
		});

		expect(new Contract(container.raw).toJSON()).to.deep.equal(container.raw);
	});
});
