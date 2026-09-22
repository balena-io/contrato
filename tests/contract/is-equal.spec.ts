/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';
import CONTRACTS from '../contracts.json';

describe('Contract isEqual', () => {
	it('should return true if the contracts are equal', () => {
		const contract1 = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		const contract2 = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		expect(Contract.isEqual(contract1, contract2)).to.be.true;
		expect(contract1.isEqual(contract2)).to.be.true;
		expect(contract2.isEqual(contract1)).to.be.true;
	});

	it('should return false if the contracts are different', () => {
		const contract1 = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		const contract2 = new Contract({
			type: 'arch.sw',
			name: 'i386',
			slug: 'i386',
		});

		expect(Contract.isEqual(contract1, contract2)).to.be.false;
		expect(contract1.isEqual(contract2)).to.be.false;
		expect(contract2.isEqual(contract1)).to.be.false;
	});

	it('should behave the same as a static function and as a method', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

		expect(Contract.isEqual(contract1, contract2)).to.equal(
			contract1.isEqual(contract2),
		);
		expect(Contract.isEqual(contract1, contract3)).to.equal(
			contract1.isEqual(contract3),
		);
	});

	it('should return true if the contract is compared with itself', () => {
		const contract = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);

		expect(contract.isEqual(contract)).to.be.true;
	});

	it('should ignore the order of the properties of the source object', () => {
		const contract1 = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		const contract2 = new Contract({
			slug: 'armv7hf',
			name: 'armv7hf',
			type: 'arch.sw',
		});

		expect(contract1.isEqual(contract2)).to.be.true;
	});

	it('should return false if the contracts have different children', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);

		contract1.addChild(new Contract(CONTRACTS['arch.sw'].amd64.object));
		contract2.addChild(new Contract(CONTRACTS['arch.sw'].i386.object));

		expect(contract1.isEqual(contract2)).to.be.false;
	});

	it('should ignore the order in which children of different slugs were added', () => {
		const amd64 = new Contract(CONTRACTS['arch.sw'].amd64.object);
		const i386 = new Contract(CONTRACTS['arch.sw'].i386.object);

		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);

		contract1.addChildren([amd64, i386]);
		contract2.addChildren([i386, amd64]);

		expect(contract1.isEqual(contract2)).to.be.true;
	});

	it('should consider the order in which children of the same slug were added', () => {
		// Same type and slug means the children are kept as an ordered list,
		// so the insertion order is part of the contract's content
		const wheezy = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const jessie = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		const contract1 = new Contract({ type: 'foo', slug: 'bar' });
		const contract2 = new Contract({ type: 'foo', slug: 'bar' });

		contract1.addChildren([wheezy, jessie]);
		contract2.addChildren([jessie, wheezy]);

		expect(contract1.isEqual(contract2)).to.be.false;
	});

	it('should return true for a clone', () => {
		const contract = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		contract.addChild(new Contract(CONTRACTS['arch.sw'].amd64.object));

		expect(contract.isEqual(contract.clone())).to.be.true;
	});

	it('should return true for a contract rebuilt from its JSON', () => {
		const contract = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		contract.addChild(new Contract(CONTRACTS['arch.sw'].amd64.object));

		expect(contract.isEqual(new Contract(contract.toJSON()))).to.be.true;
	});

	it('should return false if only one of the contracts has aliases', () => {
		const contract1 = new Contract({
			type: 'hw.device-type',
			name: 'Raspberry Pi',
			slug: 'raspberrypi',
		});

		const contract2 = new Contract({
			type: 'hw.device-type',
			name: 'Raspberry Pi',
			slug: 'raspberrypi',
			aliases: ['rpi', 'raspberry-pi'],
		});

		expect(contract1.isEqual(contract2)).to.be.false;
	});

	it('should follow mutations on either side', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const child = new Contract(CONTRACTS['arch.sw'].amd64.object);

		contract1.addChild(child);
		expect(contract1.isEqual(contract2)).to.be.false;

		contract2.addChild(child);
		expect(contract1.isEqual(contract2)).to.be.true;

		contract2.removeChild(child);
		expect(contract1.isEqual(contract2)).to.be.false;
	});

	it('should return false if the contracts differ in a nested child', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);

		const blob1 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		const blob2 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		blob2.addChild(new Contract(CONTRACTS['arch.sw'].amd64.object));

		contract1.addChild(blob1);
		contract2.addChild(blob2);

		expect(contract1.isEqual(contract2)).to.be.false;
	});
});
