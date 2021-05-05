/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';
import { hashObject } from '../../lib/hash';
import CONTRACTS from '../contracts.json';

const SKELETON = {
	type: 'foo',
	slug: 'bar',
};

it('should delete a contract from a set of contracts', () => {
	const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
	const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
	const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2, contract3]);
	container.removeChild(contract2);

	const expected = new Contract(SKELETON);
	expected.addChildren([contract1, contract3]);
	expect(container).to.deep.equal(expected);
});

it('should ignore contracts that are not in the set', () => {
	const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
	const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
	const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2, contract3]);
	container.removeChild(
		new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
	);

	const expected = new Contract(SKELETON);
	expected.addChildren([contract1, contract2, contract3]);
	expect(container).to.deep.equal(expected);
});

it('should remove a slug object if it becomes empty after the removal', () => {
	const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
	const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2]);
	container.removeChild(contract1);

	const expected = new Contract(SKELETON);
	expected.addChildren([contract2]);
	expect(container).to.deep.equal(expected);
});

it('should correctly handle number versions', () => {
	const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
	const contract2 = new Contract(CONTRACTS['sw.os'].fedora['24'].object);
	const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2, contract3]);
	container.removeChild(contract2);

	const expected = new Contract(SKELETON);
	expected.addChildren([contract1, contract3]);
	expect(container).to.deep.equal(expected);
});

it('should ignore an invalid version of an existing contract', () => {
	const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
	const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
	const contract3 = new Contract(CONTRACTS['sw.os'].debian.sid.object);

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2]);
	container.removeChild(contract3);

	const expected = new Contract(SKELETON);
	expected.addChildren([contract1, contract2]);
	expect(container).to.deep.equal(expected);
});

it('should take versions into account before removing a contract from the slug object', () => {
	const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
	const contract2 = new Contract(CONTRACTS['sw.os'].fedora['24'].object);
	const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2]);
	container.removeChild(contract3);

	const expected = new Contract(SKELETON);
	expected.addChildren([contract1, contract2]);
	expect(container).to.deep.equal(expected);
});

it('should take versions into account before removing a contract from the type object', () => {
	const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
	const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

	const container = new Contract(SKELETON);
	container.addChildren([contract1]);
	container.removeChild(contract2);

	const expected = new Contract(SKELETON);
	expected.addChildren([contract1]);
	expect(container).to.deep.equal(expected);
});

it('should remove a type object if it becomes empty after the removal', () => {
	const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
	const contract2 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2]);
	container.removeChild(contract1);

	const expected = new Contract(SKELETON);
	expected.addChildren([contract2]);
	expect(container).to.deep.equal(expected);
});

it('should return the object instance', () => {
	const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
	const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
	const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2, contract3]);
	expect(container.removeChild(contract2)).to.deep.equal(container);
});

it('should return the object instance even if the type does not exist', () => {
	const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
	const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
	const contract3 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2]);
	expect(container.removeChild(contract3)).to.deep.equal(container);
});

it('should remove a contract with aliases', () => {
	const contract1 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
	const contract2 = new Contract({
		type: 'hw.device-type',
		name: 'Raspberry Pi',
		slug: 'raspberrypi',
		aliases: ['rpi', 'raspberry-pi'],
	});

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2]);
	container.removeChild(contract2);

	const expected = new Contract(SKELETON);
	expected.addChildren([contract1]);
	expect(container).to.deep.equal(expected);
});

it('should remove a contract with aliases when there is another aliased contract of the same type', () => {
	const contract1 = new Contract({
		type: 'hw.device-type',
		name: 'Intel NUC',
		slug: 'intel-nuc',
		aliases: ['nuc'],
	});

	const contract2 = new Contract({
		type: 'hw.device-type',
		name: 'Raspberry Pi',
		slug: 'raspberrypi',
		aliases: ['rpi', 'raspberry-pi'],
	});

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2]);
	container.removeChild(contract2);

	const expected = new Contract(SKELETON);
	expected.addChildren([contract1]);
	expect(container).to.deep.equal(expected);
});

it('should remove a contract with aliases when there are two aliased contracts of the same type', () => {
	const contract1 = new Contract({
		type: 'hw.device-type',
		name: 'Intel NUC',
		slug: 'intel-nuc',
		aliases: ['nuc'],
	});

	const contract2 = new Contract({
		type: 'hw.device-type',
		name: 'Raspberry Pi',
		slug: 'raspberrypi',
		aliases: ['rpi', 'raspberry-pi'],
	});

	const contract3 = new Contract({
		type: 'hw.device-type',
		name: 'Intel Edison',
		slug: 'intel-edison',
		aliases: ['edison'],
	});

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2, contract3]);
	container.removeChild(contract2);

	const expected = new Contract(SKELETON);
	expected.addChildren([contract1, contract3]);
	expect(container).to.deep.equal(expected);
});

it('should re-hash the universe', () => {
	const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
	const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
	const contract3 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2, contract3]);
	const oldHash = container.metadata.hash;
	container.removeChild(contract3);

	expect(container.metadata.hash).to.not.equal(oldHash);
	expect(container.metadata.hash).to.equal(hashObject(container.raw));
});

it('should not re-hash the universe if the rehash option is false', () => {
	const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
	const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
	const contract3 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

	const container = new Contract(SKELETON);
	container.addChildren([contract1, contract2, contract3]);
	const oldHash = container.metadata.hash;
	container.removeChild(contract3, {
		rehash: false,
	});

	expect(container.metadata.hash).to.equal(oldHash);
});
