import index from '@/pages/index.vue'
import { shallowMount } from '@vue/test-utils'

describe('index', () => {
  test('is a Vue instance', () => {
    const wrapper = shallowMount(index, {
      data() {
        return {
          page: {
            title: 'Home',
          },
        }
      },
    })
    expect(wrapper.vm).toBeTruthy()
  })
})
