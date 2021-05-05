/*
 * Copyright 2017 resin.io
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *    http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

'use strict'

// @ts-expect-error ts-migrate(2451) FIXME: Cannot redeclare block-scoped variable 'ava'.
const ava = require('ava')

// @ts-expect-error ts-migrate(2451) FIXME: Cannot redeclare block-scoped variable '_'.
const _ = require('lodash')

// @ts-expect-error ts-migrate(2451) FIXME: Cannot redeclare block-scoped variable 'Contract'.
const Contract = require('../../../lib/contract')

// @ts-expect-error ts-migrate(2451) FIXME: Cannot redeclare block-scoped variable 'Blueprint'... Remove this comment to see the full error message
const Blueprint = require('../../../lib/blueprint')

// @ts-expect-error ts-migrate(2451) FIXME: Cannot redeclare block-scoped variable 'hash'.
const hash = require('../../../lib/hash')

_.each([
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
  'requirements-simple-2-aliases'
], (testName) => {
  // @ts-expect-error ts-migrate(2580) FIXME: Cannot find name 'require'. Do you need to install... Remove this comment to see the full error message
  const testCase = require(`./${testName}.json`)

  ava(testName, (test) => {
    const contracts = _.flatMap(testCase.universe, Contract.build)
    const container = new Contract({
      type: 'meta.universe'
    })

    container.addChildren(contracts)

    const blueprint = new Blueprint(testCase.blueprint.layout, testCase.blueprint.skeleton)
    const result = blueprint.reproduce(container)
    test.deepEqual(testCase.contexts, _.invokeMap(result, 'toJSON'))
  })
})

ava('should consider the skeleton when computing the hashes', (test) => {
  const blueprint = new Blueprint({
    'hw.device-type': 1
  }, {
    type: 'hw.context.device-type',
    foo: 'bar',
    bar: {
      baz: 1
    }
  })

  const contract1 = new Contract({
    type: 'hw.device-type',
    name: 'Intel Edison',
    slug: 'intel-edison'
  })

  const contract2 = new Contract({
    type: 'hw.device-type',
    name: 'Intel NUC',
    slug: 'intel-nuc'
  })

  const container = new Contract({
    type: 'meta.universe'
  })

  container.addChildren([ contract1, contract2 ])
  const contexts = blueprint.reproduce(container)

  for (const context of contexts) {
    test.is(context.metadata.hash, hash.hashObject(context.raw))
  }
})
