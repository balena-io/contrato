/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';
import CONTRACTS from '../contracts.json';

describe('Contract satisfiesChildContract', () => {
	it('should return true given an empty contract and a contract without requirements', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		expect(
			contract.satisfiesChildContract(
				new Contract({
					type: 'test',
					slug: 'foo',
					name: 'Foo',
					version: '1.2.3',
				}),
			),
		).to.be.true;
	});

	it('should return false given an empty contract and a contract with requirements', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		expect(
			contract.satisfiesChildContract(
				new Contract({
					type: 'test',
					slug: 'foo',
					name: 'Foo',
					version: '1.2.3',
					requires: [
						{
							type: 'sw.arch',
							slug: 'amd64',
						},
					],
				}),
			),
		).to.be.false;
	});

	it('should return true given a contract without requirements', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					type: 'test',
					slug: 'foo',
					name: 'Foo',
					version: '1.2.3',
				}),
			),
		).to.be.true;
	});

	it('should return true given one fulfilled requirement', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							slug: 'debian',
							version: 'wheezy',
							type: 'sw.os',
						},
					],
				}),
			),
		).to.be.true;
	});

	it('should return true given two fulfilled requirements', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							slug: 'debian',
							version: 'wheezy',
							type: 'sw.os',
						},
						{
							slug: 'artik10',
							type: 'hw.device-type',
						},
					],
				}),
			),
		).to.be.true;
	});

	it('should return false given one unfulfilled requirement', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							slug: 'void',
							type: 'sw.os',
						},
					],
				}),
			),
		).to.be.false;
	});

	it('should return false given two requirements where one is not fulfilled', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							slug: 'void',
							type: 'sw.os',
						},
						{
							slug: 'artik10',
							type: 'hw.device-type',
						},
					],
				}),
			),
		).to.be.false;
	});

	it('should return true given no requirements in a disjunction', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							or: [],
						},
					],
				}),
			),
		).to.be.true;
	});

	it('should return false given a partially unfulfilled not operator', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							not: [
								{
									slug: CONTRACTS['sw.os'].fedora['24'].object.slug,
									type: 'sw.os',
								},
								{
									slug: CONTRACTS['sw.os'].debian.wheezy.object.slug,
									type: 'sw.os',
								},
							],
						},
					],
				}),
			),
		).to.be.false;
	});

	it('should return false given an unfulfilled not operator', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							not: [
								{
									slug: CONTRACTS['sw.os'].debian.wheezy.object.slug,
									type: 'sw.os',
								},
							],
						},
					],
				}),
			),
		).to.be.false;
	});

	it('should return false given a fulfilled not operator', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							not: [
								{
									slug: 'foo-bar',
									type: 'sw.os',
								},
							],
						},
					],
				}),
			),
		).to.be.true;
	});

	it('should return true given an empty not operator', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							not: [],
						},
					],
				}),
			),
		).to.be.true;
	});

	it('should return false given two unfulfilled requirements', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							slug: 'void',
							type: 'sw.os',
						},
						{
							slug: 'raspberry-pi',
							type: 'hw.device-type',
						},
					],
				}),
			),
		).to.be.false;
	});

	it('should return true given one fulfilled requirement in a disjunction', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							or: [
								{
									slug: 'debian',
									type: 'sw.os',
								},
							],
						},
					],
				}),
			),
		).to.be.true;
	});

	it('should return true given one fulfilled and one unfulfilled requirement in a disjunction', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							or: [
								{
									slug: 'debian',
									type: 'sw.os',
								},
								{
									slug: 'void',
									type: 'sw.os',
								},
							],
						},
					],
				}),
			),
		).to.be.true;
	});

	it('should return false given one unfulfilled requirement in a disjunction', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'stack',
					requires: [
						{
							or: [
								{
									slug: 'void',
									type: 'sw.os',
								},
							],
						},
					],
				}),
			),
		).to.be.false;
	});

	it('should return false given an empty disjunction and an unfulfilled requirement', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'stack',
					requires: [
						{
							or: [],
						},
						{
							slug: 'void',
							type: 'sw.os',
						},
					],
				}),
			),
		).to.be.false;
	});

	it('should return false given an fulfilled disjunction and an unfulfilled requirement', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'stack',
					requires: [
						{
							or: [
								{
									type: 'sw.os',
									slug: 'void',
								},
								{
									type: 'sw.os',
									slug: 'debian',
								},
							],
						},
						{
							slug: 'raspberry-pi',
							type: 'hw.device-type',
						},
					],
				}),
			),
		).to.be.false;
	});

	it('should be able to specify a single type', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							slug: 'debian',
							type: 'sw.os',
						},
						{
							slug: 'hello',
							type: 'test',
						},
					],
				}),
				{
					types: new Set(['sw.os']),
				},
			),
		).to.be.true;
	});

	it('should be able to specify multiple types', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							slug: 'debian',
							type: 'sw.os',
						},
						{
							or: [
								{
									type: 'hw.device-type',
									slug: 'artik10',
								},
								{
									type: 'hw.device-type',
									slug: 'raspberry-pi',
								},
							],
						},
						{
							slug: 'hello',
							type: 'test',
						},
					],
				}),
				{
					types: new Set(['sw.os', 'hw.device-type']),
				},
			),
		).to.be.true;
	});

	it('should return false given one unfulfilled requirement selected type', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'stack',
					requires: [
						{
							slug: 'void',
							type: 'sw.os',
						},
					],
				}),
				{
					types: new Set(['sw.os']),
				},
			),
		).to.be.false;
	});

	it('should return true given one unfulfilled requirement in a disjunction of a non-selected type', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract(CONTRACTS['hw.device-type'].artik10.object),
		]);

		expect(
			contract.satisfiesChildContract(
				new Contract({
					name: 'Debian',
					slug: 'debian',
					type: 'sw.os',
					requires: [
						{
							or: [
								{
									type: 'hw.device-type',
									slug: 'intel-edison',
								},
								{
									type: 'hw.device-type',
									slug: 'raspberry-pi',
								},
							],
						},
					],
				}),
				{
					types: new Set(['arch.sw']),
				},
			),
		).to.be.true;
	});

	it('should return true given two fulfilled requirements from a context with a composite contract', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const contract1 = new Contract({
			type: 'meta.composite',
			slug: 'test',
		});

		contract1.addChildren([
			new Contract({
				type: 'sw.os',
				slug: 'debian',
				version: 'wheezy',
			}),
			new Contract({
				type: 'arch.sw',
				slug: 'amd64',
				version: '1',
			}),
		]);

		container.addChild(contract1);

		expect(
			container.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							slug: 'debian',
							type: 'sw.os',
						},
						{
							or: [
								{
									slug: 'amd64',
									type: 'arch.sw',
								},
								{
									slug: 'i386',
									type: 'arch.sw',
								},
							],
						},
					],
				}),
			),
		).to.be.true;
	});

	it('should return true given two fulfilled requirements from a context declaring capabilities with `provides`', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const contract1 = new Contract({
			type: 'meta.context',
			slug: 'test',
			provides: [
				{
					type: 'sw.os',
					slug: 'debian',
					version: 'wheezy',
				},
				{
					type: 'arch.sw',
					slug: 'amd64',
					version: '1',
				},
			],
		});

		container.addChild(contract1);

		expect(
			container.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							slug: 'debian',
							type: 'sw.os',
						},
						{
							or: [
								{
									slug: 'amd64',
									type: 'arch.sw',
								},
								{
									slug: 'i386',
									type: 'arch.sw',
								},
							],
						},
					],
				}),
			),
		).to.be.true;
	});

	it('should return true given one fulfilled requirements from a context declaring capabilities with `provides` and one selected type', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const contract1 = new Contract({
			type: 'meta.context',
			slug: 'test',
			provides: [
				{
					type: 'sw.os',
					slug: 'debian',
					version: 'wheezy',
				},
			],
		});

		container.addChild(contract1);

		expect(
			container.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							slug: 'debian',
							type: 'sw.os',
						},
						{
							or: [
								{
									slug: 'amd64',
									type: 'arch.sw',
								},
								{
									slug: 'i386',
									type: 'arch.sw',
								},
							],
						},
					],
				}),
				{ types: new Set(['sw.os']) },
			),
		).to.be.true;
	});

	it('should return false given only one fulfilled requirements from a context declaring capabilities with `provides`', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const contract1 = new Contract({
			type: 'meta.context',
			slug: 'test',
			provides: [
				{
					type: 'sw.os',
					slug: 'debian',
					version: 'wheezy',
				},
			],
		});

		container.addChild(contract1);

		const child = new Contract({
			name: 'Node.js',
			slug: 'nodejs',
			type: 'sw.stack',
			requires: [
				{
					slug: 'debian',
					type: 'sw.os',
				},
				{
					or: [
						{
							slug: 'amd64',
							type: 'arch.sw',
						},
						{
							slug: 'i386',
							type: 'arch.sw',
						},
					],
				},
			],
		});

		expect(container.satisfiesChildContract(child)).to.be.false;
		expect(container.getNotSatisfiedChildRequirements(child)).to.have.lengthOf(
			1,
		);
	});

	it('should return false given one unfulfilled requirement from a context with a composite contract', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const contract1 = new Contract({
			type: 'meta.composite',
			slug: 'test',
		});

		contract1.addChildren([
			new Contract({
				type: 'sw.os',
				slug: 'debian',
				version: 'wheezy',
			}),
			new Contract({
				type: 'arch.sw',
				slug: 'amd64',
				version: '1',
			}),
		]);

		container.addChild(contract1);

		expect(
			container.satisfiesChildContract(
				new Contract({
					name: 'Node.js',
					slug: 'nodejs',
					type: 'sw.stack',
					requires: [
						{
							slug: 'fedora',
							type: 'sw.os',
						},
						{
							or: [
								{
									slug: 'amd64',
									type: 'arch.sw',
								},
								{
									slug: 'i386',
									type: 'arch.sw',
								},
							],
						},
					],
				}),
			),
		).to.be.false;
	});

	it(
		'should return true given one ignored unfulfilled requirement ' +
			'on a context with a composite contract',
		() => {
			const container = new Contract({
				type: 'foo',
				slug: 'bar',
			});

			const contract1 = new Contract({
				type: 'meta.composite',
				slug: 'test',
			});

			contract1.addChildren([
				new Contract({
					type: 'sw.os',
					slug: 'debian',
					version: 'wheezy',
				}),
				new Contract({
					type: 'arch.sw',
					slug: 'amd64',
					version: '1',
				}),
			]);

			container.addChild(contract1);

			expect(
				container.satisfiesChildContract(
					new Contract({
						name: 'Node.js',
						slug: 'nodejs',
						type: 'sw.stack',
						requires: [
							{
								slug: 'fedora',
								type: 'sw.os',
							},
							{
								or: [
									{
										slug: 'amd64',
										type: 'arch.sw',
									},
									{
										slug: 'i386',
										type: 'arch.sw',
									},
								],
							},
						],
					}),
					{
						types: new Set(['arch.sw']),
					},
				),
			).to.be.true;
		},
	);

	it('should return true given a fulfilled context as an argument', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const contract1 = new Contract({
			type: 'arch.sw',
			slug: 'amd64',
		});

		const contract2 = new Contract({
			type: 'meta.composite',
			slug: 'test',
		});

		contract2.addChildren([
			new Contract({
				type: 'sw.os',
				slug: 'debian',
				version: 'wheezy',
				requires: [
					{
						type: 'arch.sw',
						slug: 'amd64',
					},
				],
			}),
		]);

		container.addChild(contract1);

		expect(container.satisfiesChildContract(contract2)).to.be.true;
	});

	it('should return false given a unfulfilled context as an argument', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const contract1 = new Contract({
			type: 'arch.sw',
			slug: 'amd64',
		});

		const contract2 = new Contract({
			type: 'meta.composite',
			slug: 'test',
		});

		contract2.addChildren([
			new Contract({
				type: 'sw.os',
				slug: 'debian',
				version: 'wheezy',
				requires: [
					{
						type: 'arch.sw',
						slug: 'armv7hf',
					},
				],
			}),
		]);

		container.addChild(contract1);

		expect(container.satisfiesChildContract(contract2)).to.be.false;
	});
});
