/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';

import CONTRACTS from '../contracts.json';

describe('build children tree', () => {
	it('should build a tree with one children', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		container.addChild(contract1);

		expect(container.raw().children).to.deep.equal({
			sw: {
				os: contract1.raw(),
			},
		});

		expect(container.getChildByHash(contract1.hash())).to.deep.equal(contract1);
		expect(container.getChildrenByType('sw.os')).to.deep.equal([contract1]);
		expect(
			container.findChildren(
				Contract.createMatcher({ type: 'sw.os', slug: 'debian' }),
			),
		).to.deep.equal([contract1]);
	});

	it('should build a tree with two contracts of different types', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);
		container.addChild(contract2);

		expect(container.raw().children).to.deep.equal({
			sw: {
				os: contract1.raw(),
				blob: contract2.raw(),
			},
		});

		expect(container.getChildByHash(contract1.hash())).to.deep.equal(contract1);
		expect(container.getChildByHash(contract2.hash())).to.deep.equal(contract2);
		expect(container.getChildrenByType('sw.os')).to.deep.equal([contract1]);
		expect(container.getChildrenByType('sw.blob')).to.deep.equal([contract2]);
	});

	it('should build a tree with two contracts of the same type', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);
		container.addChild(contract2);

		expect(container.raw().children).to.deep.equal({
			sw: {
				os: {
					debian: contract1.raw(),
					fedora: contract2.raw(),
				},
			},
		});

		expect(container.getChildByHash(contract1.hash())).to.deep.equal(contract1);
		expect(container.getChildByHash(contract2.hash())).to.deep.equal(contract2);
		expect(container.getChildrenByType('sw.os')).to.have.deep.members([
			contract1,
			contract2,
		]);
		expect(
			container.findChildren(
				Contract.createMatcher({ type: 'sw.os', slug: 'debian' }),
			),
		).to.deep.equal([contract1]);
		expect(
			container.findChildren(
				Contract.createMatcher({ type: 'sw.os', slug: 'fedora' }),
			),
		).to.deep.equal([contract2]);
	});

	it('should build a tree with two versions of the same slug', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);
		container.addChild(contract2);

		expect(container.raw().children).to.deep.equal({
			sw: {
				os: {
					debian: [contract1.raw(), contract2.raw()],
				},
			},
		});

		expect(container.getChildByHash(contract1.hash())).to.deep.equal(contract1);
		expect(container.getChildByHash(contract2.hash())).to.deep.equal(contract2);
		expect(container.getChildrenByType('sw.os')).to.have.deep.members([
			contract1,
			contract2,
		]);
		expect(
			container.findChildren(
				Contract.createMatcher({ type: 'sw.os', slug: 'debian' }),
			),
		).to.have.deep.members([contract1, contract2]);
	});

	it('should create a tree of two variants of the same contract', () => {
		const contract1 = new Contract({
			type: 'sw.os',
			slug: 'debian',
			version: 'wheezy',
			requires: [
				{
					type: 'arch.sw',
					slug: 'amd64',
				},
			],
		});

		const contract2 = new Contract({
			type: 'sw.os',
			slug: 'debian',
			version: 'wheezy',
			requires: [
				{
					type: 'arch.sw',
					slug: 'armv7hf',
				},
			],
		});

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);
		container.addChild(contract2);

		expect(container.raw().children).to.deep.equal({
			sw: {
				os: {
					debian: [contract1.raw(), contract2.raw()],
				},
			},
		});

		expect(container.getChildByHash(contract1.hash())).to.deep.equal(contract1);
		expect(container.getChildByHash(contract2.hash())).to.deep.equal(contract2);
		expect(container.getChildrenByType('sw.os')).to.have.deep.members([
			contract1,
			contract2,
		]);
		expect(
			container.findChildren(
				Contract.createMatcher({ type: 'sw.os', slug: 'debian' }),
			),
		).to.have.deep.members([contract1, contract2]);
	});
});
