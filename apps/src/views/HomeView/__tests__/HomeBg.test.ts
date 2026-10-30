import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import HomeBg from '../HomeBg.vue'

const data = (pv: unknown, load = pv, diesel = pv) => ({
  pv: { P: { rawValue: pv } },
  load: { P: { rawValue: load } },
  diesel: { p: { rawValue: diesel } },
  ess: {},
})

describe('homepage energy flow', () => {
  it.each([null, undefined, '', ' ', '-', 'NaN', NaN, Infinity, false, {}, 0, '0'])(
    'keeps lines static for missing, invalid or zero power: %s',
    (power) => {
      const wrapper = mount(HomeBg, { props: { data: data(power) } })
      expect(wrapper.findAll('animateTransform')).toHaveLength(0)
      expect(wrapper.findAll('[data-energy-flow]')).toHaveLength(3)
      wrapper.unmount()
    },
  )

  it('routes PV and diesel generation inward and load consumption outward', () => {
    const wrapper = mount(HomeBg, { props: { data: data(20, 30, 10) } })
    expect(wrapper.find('[data-energy-flow="pv"]').attributes('data-direction')).toBe('1')
    expect(wrapper.find('[data-energy-flow="load"]').attributes('data-direction')).toBe('1')
    const diesel = wrapper.find('[data-energy-flow="diesel"]')
    expect(diesel.attributes('data-direction')).toBe('-1')
    expect(diesel.find('g').attributes('transform')).toContain('scale(-1 1)')
    expect(wrapper.findAll('animateTransform')).toHaveLength(3)
    wrapper.unmount()
  })

  it('reverses on negative readings and stops each branch when its data disappears', async () => {
    const wrapper = mount(HomeBg, { props: { data: data(20) } })
    await wrapper.setProps({ data: data(-20) })
    expect(wrapper.find('[data-energy-flow="pv"]').attributes('data-direction')).toBe('-1')
    expect(wrapper.find('[data-energy-flow="load"]').attributes('data-direction')).toBe('-1')
    expect(wrapper.find('[data-energy-flow="diesel"]').attributes('data-direction')).toBe('1')
    await wrapper.setProps({ data: data(null, 12, 0) })
    expect(wrapper.findAll('animateTransform')).toHaveLength(1)
    expect(wrapper.find('[data-energy-flow="load"] animateTransform').exists()).toBe(true)
    await wrapper.setProps({ data: data(null) })
    expect(wrapper.findAll('animateTransform')).toHaveLength(0)
    wrapper.unmount()
  })
})
