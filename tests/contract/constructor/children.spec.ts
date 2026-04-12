import { expect } from '../../chai';

import Contract from '../../../lib/contract';

describe('Contract children', () => {
	it('should take a contract with a single child', () => {
		const contract = new Contract({
			type: 'misc.collection',
			slug: 'my-collection',
			children: {
				arch: {
					sw: {
						type: 'arch.sw',
						name: 'armv7hf',
						slug: 'armv7hf',
					},
				},
			},
		});

		const children = contract.getChildren();
		expect(children).to.have.lengthOf(1);
		expect(children[0].getType()).to.equal('arch.sw');
		expect(children[0].getSlug()).to.equal('armv7hf');

		expect(contract.getChildrenTypes()).to.deep.equal(new Set(['arch.sw']));

		expect(contract.raw).to.deep.equal({
			type: 'misc.collection',
			slug: 'my-collection',
			children: {
				arch: {
					sw: {
						type: 'arch.sw',
						name: 'armv7hf',
						slug: 'armv7hf',
					},
				},
			},
		});

		expect(new Contract(contract.raw).raw).to.deep.equal(contract.raw);
	});

	it('should take a contract with two children of the same type', () => {
		const contract = new Contract({
			type: 'misc.collection',
			slug: 'my-collection',
			children: {
				arch: {
					sw: {
						armv7hf: {
							type: 'arch.sw',
							name: 'armv7hf',
							slug: 'armv7hf',
						},
						armel: {
							type: 'arch.sw',
							name: 'armel',
							slug: 'armel',
						},
					},
				},
			},
		});

		const children = contract.getChildren();
		expect(children).to.have.lengthOf(2);
		expect(contract.getChildrenTypes()).to.deep.equal(new Set(['arch.sw']));

		const slugs = children.map((c) => c.getSlug()).sort();
		expect(slugs).to.deep.equal(['armel', 'armv7hf']);

		expect(contract.raw).to.deep.equal({
			type: 'misc.collection',
			slug: 'my-collection',
			children: {
				arch: {
					sw: {
						armv7hf: {
							type: 'arch.sw',
							name: 'armv7hf',
							slug: 'armv7hf',
						},
						armel: {
							type: 'arch.sw',
							name: 'armel',
							slug: 'armel',
						},
					},
				},
			},
		});

		expect(new Contract(contract.raw).raw).to.deep.equal(contract.raw);
	});

	it('should take a contract with two children of the same type and slug', () => {
		const contract = new Contract({
			type: 'misc.collection',
			slug: 'my-collection',
			children: {
				sw: {
					distro: {
						debian: [
							{
								type: 'sw.distro',
								name: 'debian',
								version: 'wheezy',
								slug: 'debian',
							},
							{
								type: 'sw.distro',
								name: 'debian',
								version: 'jessie',
								slug: 'debian',
							},
						],
					},
				},
			},
		});

		const children = contract.getChildren();
		expect(children).to.have.lengthOf(2);
		expect(contract.getChildrenTypes()).to.deep.equal(new Set(['sw.distro']));

		const versions = children.map((c) => c.getVersion()).sort();
		expect(versions).to.deep.equal(['jessie', 'wheezy']);

		const json = contract.raw;
		const debianArray = json.children.sw.distro.debian as any[];
		expect(debianArray).to.have.lengthOf(2);
		const debianVersions = debianArray.map((c: any) => c.version).sort();
		expect(debianVersions).to.deep.equal(['jessie', 'wheezy']);

		expect(new Contract(contract.raw).raw).to.deep.equal(contract.raw);
	});

	it('should take a contract with two children of different types', () => {
		const contract = new Contract({
			type: 'misc.collection',
			slug: 'my-collection',
			children: {
				arch: {
					sw: {
						type: 'arch.sw',
						name: 'armv7hf',
						slug: 'armv7hf',
					},
				},
				sw: {
					distro: {
						type: 'sw.distro',
						name: 'debian',
						version: 'wheezy',
						slug: 'debian',
					},
				},
			},
		});

		const children = contract.getChildren();
		expect(children).to.have.lengthOf(2);
		expect(contract.getChildrenTypes()).to.deep.equal(
			new Set(['arch.sw', 'sw.distro']),
		);

		expect(contract.raw).to.deep.equal({
			type: 'misc.collection',
			slug: 'my-collection',
			children: {
				arch: {
					sw: {
						type: 'arch.sw',
						name: 'armv7hf',
						slug: 'armv7hf',
					},
				},
				sw: {
					distro: {
						type: 'sw.distro',
						name: 'debian',
						version: 'wheezy',
						slug: 'debian',
					},
				},
			},
		});

		expect(new Contract(contract.raw).raw).to.deep.equal(contract.raw);
	});
});
