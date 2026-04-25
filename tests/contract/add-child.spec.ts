import { expect } from '../chai';

import Contract from '../../lib/contract';
import CONTRACTS from '../contracts.json';

describe('Contract addChild', () => {
	it('should add a contract to a contract without children', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		container.addChild(contract1);

		expect(container.getChildren()).to.have.lengthOf(1);
		expect(container.getChildrenTypes()).to.deep.equal(new Set(['sw.os']));
		expect(container.raw).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: contract1.raw,
				},
			},
		});
	});

	it('should add two contracts of different types', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);
		container.addChild(contract2);

		expect(container.getChildren()).to.have.lengthOf(2);
		expect(container.getChildrenTypes()).to.deep.equal(
			new Set(['sw.os', 'sw.blob']),
		);

		expect(container.raw).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: contract1.raw,
					blob: contract2.raw,
				},
			},
		});
	});

	it('should not add a contract twice', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);
		container.addChild(contract1);

		expect(container.getChildren()).to.have.lengthOf(1);
		expect(container.raw).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: contract1.raw,
				},
			},
		});
	});

	it('should add two contracts of same type but different slugs', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);
		container.addChild(contract2);

		expect(container.getChildren()).to.have.lengthOf(2);
		expect(container.raw).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: {
						debian: contract1.raw,
						fedora: contract2.raw,
					},
				},
			},
		});
	});

	it('should add a new version of an existing contract', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);
		container.addChild(contract2);

		expect(container.getChildren()).to.have.lengthOf(2);
		expect(container.raw).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: {
						debian: [contract1.raw, contract2.raw],
					},
				},
			},
		});
	});

	it('should return the instance', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		expect(container.addChild(contract1)).to.equal(container);
	});

	it('should change the hash of the parent contract', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const hash = container.hash();
		container.addChild(contract1);
		expect(container.hash()).to.not.equal(hash);
	});
});
