/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../../chai';
import * as _ from 'lodash';

import Contract from '../../../lib/contract';
import Blueprint from '../../../lib/blueprint';

describe('Blueprint reproduce', () => {
	_.each(
		[
			'1-to-1',
			'1-to-1-skeleton-no-template',
			'1-to-1-skeleton-template',
			'1-to-2',
			'1-to-all',
			'cartesian-simple-2',
			'reference-single',
			'reference-multiple',
			'reference-nested',
			'requirements-or-2',
			'requirements-simple-2',
			'requirements-simple-2-aliases',
		],
		(testName) => {
			// eslint-disable-next-line @typescript-eslint/no-require-imports
			const testCase = require(`./${testName}.json`);

			it(testName, () => {
				const contracts = _.flatMap(testCase.universe, Contract.build);
				const container = new Contract({
					type: 'meta.universe',
				});

				container.addChildren(contracts);

				const blueprint = new Blueprint(
					testCase.blueprint.layout,
					testCase.blueprint.skeleton,
				);
				const result = [...blueprint.reproduce(container)];
				expect(testCase.contexts).to.deep.equal(result.map((r) => r.toJSON()));
			});
		},
	);

	it('should consider the skeleton when computing the hashes', () => {
		const skeleton = {
			type: 'hw.context.device-type',
			foo: 'bar',
			bar: {
				baz: 1,
			},
		};

		const blueprint = new Blueprint(
			{
				'hw.device-type': 1,
			},
			skeleton,
		);

		const contract1 = new Contract({
			type: 'hw.device-type',
			name: 'Intel Edison',
			slug: 'intel-edison',
		});

		const contract2 = new Contract({
			type: 'hw.device-type',
			name: 'Intel NUC',
			slug: 'intel-nuc',
		});

		const container = new Contract({
			type: 'meta.universe',
		});

		container.addChildren([contract1, contract2]);
		const contexts = Array.from(blueprint.reproduce(container));

		const derivedContract1 = new Contract(skeleton).addChild(contract1);
		const derivedContract2 = new Contract(skeleton).addChild(contract2);

		expect(contexts).to.have.length(2);
		expect(contexts[0].hash()).to.equal(derivedContract1.hash());
		expect(contexts[1].hash()).to.equal(derivedContract2.hash());
	});
});
