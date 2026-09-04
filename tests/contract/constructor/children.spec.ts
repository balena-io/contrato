/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

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

		expect(contract.raw()).to.deep.equal({
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

		expect(new Contract(contract.raw())).to.deep.equal(contract);

		const child = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		expect(contract.getChildrenByType('arch.sw')).to.deep.equal([child]);
		expect(
			contract.findChildren(
				Contract.createMatcher({ type: 'arch.sw', slug: 'armv7hf' }),
			),
		).to.deep.equal([child]);
		expect(contract.getChildByHash(child.hash())).to.deep.equal(child);
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

		expect(contract.raw()).to.deep.equal({
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

		expect(new Contract(contract.raw())).to.deep.equal(contract);

		const armv7hf = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});
		const armel = new Contract({
			type: 'arch.sw',
			name: 'armel',
			slug: 'armel',
		});

		expect(contract.getChildrenByType('arch.sw')).to.have.deep.members([
			armv7hf,
			armel,
		]);
		expect(
			contract.findChildren(
				Contract.createMatcher({ type: 'arch.sw', slug: 'armv7hf' }),
			),
		).to.deep.equal([armv7hf]);
		expect(
			contract.findChildren(
				Contract.createMatcher({ type: 'arch.sw', slug: 'armel' }),
			),
		).to.deep.equal([armel]);
		expect(contract.getChildByHash(armv7hf.hash())).to.deep.equal(armv7hf);
		expect(contract.getChildByHash(armel.hash())).to.deep.equal(armel);
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

		expect(contract.raw()).to.deep.equal({
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

		expect(new Contract(contract.raw())).to.deep.equal(contract);

		const wheezy = new Contract({
			type: 'sw.distro',
			name: 'debian',
			version: 'wheezy',
			slug: 'debian',
		});
		const jessie = new Contract({
			type: 'sw.distro',
			name: 'debian',
			version: 'jessie',
			slug: 'debian',
		});

		expect(contract.getChildrenByType('sw.distro')).to.have.deep.members([
			wheezy,
			jessie,
		]);
		expect(
			contract.findChildren(
				Contract.createMatcher({ type: 'sw.distro', slug: 'debian' }),
			),
		).to.have.deep.members([wheezy, jessie]);
		expect(contract.getChildByHash(wheezy.hash())).to.deep.equal(wheezy);
		expect(contract.getChildByHash(jessie.hash())).to.deep.equal(jessie);
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

		expect(contract.raw()).to.deep.equal({
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

		expect(new Contract(contract.raw())).to.deep.equal(contract);

		const arch = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});
		const distro = new Contract({
			type: 'sw.distro',
			name: 'debian',
			version: 'wheezy',
			slug: 'debian',
		});

		expect(contract.getChildrenByType('arch.sw')).to.deep.equal([arch]);
		expect(contract.getChildrenByType('sw.distro')).to.deep.equal([distro]);
		expect(
			contract.findChildren(
				Contract.createMatcher({ type: 'arch.sw', slug: 'armv7hf' }),
			),
		).to.deep.equal([arch]);
		expect(
			contract.findChildren(
				Contract.createMatcher({ type: 'sw.distro', slug: 'debian' }),
			),
		).to.deep.equal([distro]);
		expect(contract.getChildByHash(arch.hash())).to.deep.equal(arch);
		expect(contract.getChildByHash(distro.hash())).to.deep.equal(distro);
	});

	it('should reject overlapping types whatever the sibling count', () => {
		for (const siblings of [
			[{ type: 'sw.os', slug: 'debian' }],
			[
				{ type: 'sw.os', slug: 'debian' },
				{ type: 'sw.os', slug: 'fedora' },
			],
		]) {
			expect(
				() =>
					new Contract({
						type: 'meta.universe',
						slug: 'universe',
						children: [...siblings, { type: 'sw.os.kernel', slug: 'linux' }],
					}),
			).to.throw("'sw.os' is a prefix of 'sw.os.kernel'");
		}
	});

	it('should reject a slugless child whatever the sibling count', () => {
		for (const children of [
			[{ type: 'sw.os' }],
			[{ type: 'sw.os' }, { type: 'sw.os', slug: 'debian' }],
		]) {
			expect(
				() =>
					new Contract({
						type: 'meta.universe',
						slug: 'universe',
						children,
					}),
			).to.throw("slug missing for child of type 'sw.os'");
		}
	});

	it('should reject a child with aliases but no slug', () => {
		// Aliases are additional names for a slug, not a replacement.
		expect(
			() =>
				new Contract({
					type: 'meta.universe',
					slug: 'universe',
					children: [{ type: 'sw.os', aliases: ['deb'] }],
				}),
		).to.throw("slug missing for child of type 'sw.os'");
	});

	it('should take a slugless contract as a parent', () => {
		// Only children need a slug.
		const contract = new Contract({
			type: 'meta.context',
			children: [{ type: 'sw.os', slug: 'debian' }],
		});

		expect(contract.getSlug()).to.equal(undefined);
		expect(contract.getChildren()).to.have.lengthOf(1);
	});

	it('should keep a dotted slug as a single key in the children tree', () => {
		// A slug is one key however many dots it has; only the type is a path.
		const contract = new Contract({
			type: 'meta.universe',
			slug: 'universe',
			children: [
				{ type: 'sw.os', slug: 'node.js' },
				{ type: 'sw.os', slug: 'debian.' },
			],
		});

		expect(contract.raw().children).to.deep.equal({
			sw: {
				os: {
					'node.js': { type: 'sw.os', slug: 'node.js' },
					'debian.': { type: 'sw.os', slug: 'debian.' },
				},
			},
		});
		expect(new Contract(contract.raw()).hash()).to.equal(contract.hash());
	});
});
