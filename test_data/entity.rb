class Entity < BaseEntity
  include ::Module1
  include Module2

  CONSTANT_1 = 1
  CONSTANT_2 = 2 unless defined?(CONSTANT_1)

  ARR_CONSTANT = [1, 2, 3, 4, 5].freeze
  CONSTANT_ONE, CONSTANT_TWO, CONSTANT_THREE, *CONSTANT_REST = ARR_CONSTANT

  attr_accessor :full_access_property
  attr_reader :readonly_property
  attr_write :writeonly_property

  delegate :child_entity_method, to: :child_entity

  belongs_to :parent_entity
  belongs_to :parent_entity_with_class_name, class_name: "ParentEntity::WithClassName"

  has_one :child_entity
  has_many :multichild_entity

  scope :scope1, -> { where(state: "initial") }

  def self.singleton_method(s_param1, s_param2)
    s_local_var = s_param1 + s_param2

    s_another_local_var = s_local_var - s_param2

    s_local_var = s_another_local_var + s_param1

    return s_local_var
  end

  def instance_method(i_param1, i_param2)
    i_local_var = i_param1 + i_param2

    i_another_local_var = i_local_var - i_param2

    i_local_var = i_another_local_var + i_param1

    return i_local_var
  end
end
