# frozen_string_literal: true

class A::B::Item < ActiveRecord::Base

  module ParentModule
  end

  module Tmp
    CONSTANT_1 = "qwerty".freeze
    def m1(a1, a2)
    end
  end

  include ListingWindowAdjustmentMethods
  include PreprocessMethods
  include PhotoMethods
  include AASM
  include ConsignmentMethods
  include CartMethods
  include ConsignmentListingWindow
  include PayoutWindow
  # Modules related but not included:
  # Consignment Payout & Payout Window Mgmt - Item::PayoutWindow
  include ReclaimMethods
  include RemadeMethods
  include Search::ItemAutoUpdate
  include RabbitHelpers
  include MdcPricing
  include PromofiedPricing
  include SourcingPolicyMethods
  include Item::Indexable
  include Item::Preorderable
  include Item::Priceable
  include Item::Payoutable
  include Item::TierDiscountable
  include Item::Reservable
  include Item::Lock
  include Item::Outlet
  include Indestructible
  include Item::InvOpt::FinalSale

  attr_accessor :item_state
  attr_accessor :in_cart
  attr_accessor :reserved
  attr_accessor :name
  attr_accessor :item_brand
  attr_accessor :description
  attr_accessor :orientation
  attr_accessor :department
  attr_accessor :item_size
  attr_accessor :price
  attr_accessor :budget
  attr_accessor :original_price
  attr_accessor :savings
  attr_accessor :availability
  attr_accessor :path
  attr_accessor :item_colors
  attr_accessor :web_edited
  attr_accessor :facebook
  attr_accessor :reason
  attr_accessor :pinterest
  attr_accessor :twitter
  attr_accessor :action
  attr_accessor :item_sale
  attr_accessor :clearance
  attr_accessor :first_listing_price
  attr_accessor :default_score
  attr_accessor :junior_brand
  attr_accessor :to_cache_count
  attr_accessor :auto_update_disabled
  attr_accessor :curation_id
  attr_accessor :being_relisted

  belongs_to :brand
  belongs_to :category
  belongs_to :concierge_bag
  belongs_to :original_item, ->(obj) {
    if obj.is_a?(Item)
      where.not(id: obj.id)
    else
      join_table = obj.tables.first.right
      where("items.item_number <> #{join_table}.item_number")
    end
  }, class_name: 'Item', foreign_key: :original_item_number, primary_key: :item_number
  belongs_to :product
  belongs_to :size
  belongs_to :sizing
  belongs_to :sizing_modifier
  belongs_to :warehouse, class_name: 'Ops::Warehouse'
  belongs_to :warehouse_item_setting, primary_key: :warehouse_id, foreign_key: :warehouse_id
  belongs_to :dropshipping_warehouse, class_name: 'Dropshipping::Warehouse'

  has_one :reclamation_fee
  has_one :upfront_offer
  has_one :upfront_policy_detail
  has_one :order_product, -> { unscope(where: :order_product_type_id) }
  has_one :order, -> { unscope(where: :order_type_id) }, through: :order_product
  has_one :tup_order_product
  has_one :item_price
  has_one :partner, through: :concierge_bag
  has_one :partner_campaign, through: :concierge_bag
  has_one :item_box_item_feedback
  has_one :item_box_order_product
  has_one :item_base_discount_rate_lookup_version, foreign_key: :item_number, primary_key: :item_number
  has_one :payout_detail
  has_one :item_sales_channel
  has_one :outlet_item, foreign_key: :item_number, primary_key: :item_number
  has_one :item_fresh_sale, foreign_key: :item_number, primary_key: :item_number
  has_one :supplier_bag_item, foreign_key: :original_item_number, primary_key: :original_item_number
  has_one :partner_campaign_item_coupon
  has_many :tiered_item_promotion_discount_rates, foreign_key: :item_number, primary_key: :item_number

  has_many :characteristics
  has_many :preorders
  has_many :additional_informations, class_name: "Item::AdditionalInformation"
  has_many :history, as: :historical, dependent: :destroy
  has_many :derivative_items, ->(obj) {
    if obj.is_a?(Item)
      where.not(id: obj.id)
    else
      join_table = obj.tables.first.right
      where("items.item_number <> #{join_table}.item_number")
    end
  }, class_name: 'Item', foreign_key: :original_item_number, primary_key: :item_number

  has_many :preloaded_derivative_items,
    -> { where("items.item_number <> items.original_item_number") },
    class_name: 'Item',
    foreign_key: :original_item_number,
    primary_key: :item_number

  has_many :photos, primary_key: :item_number, foreign_key: :item_number
  has_many :item_flags, primary_key: :item_number, foreign_key: :item_number
  has_many :item_scores
  has_many :item_experiments
  has_many :item_experiment_pools
  has_many :experiment_pools, through: :item_experiment_pools
  has_many :item_pails
  has_many :pails, through: :item_pails
  has_many :auto_issue_stats, foreign_key: :item_number, primary_key: :item_number
  has_many :item_inventory_statuses, primary_key: :item_number, foreign_key: :item_number
  has_many :item_transfers
  has_many :item_department_tags
  has_many :department_tags, through: :item_department_tags
  has_many :item_search_tags
  has_many :search_tags, through: :item_search_tags
  has_one :time_spent_by_state, class_name: 'Item::TimeSpentByState'
  has_many :item_price_markdowns, primary_key: :item_number, foreign_key: :item_number
  has_many :payout_bonuses, class_name: 'PayoutBonus'
  has_many :consignment_payout_records, class_name: 'ConsignmentPayout'
  has_one  :primary_consignment_payout_record, -> { where(version: [ConsignmentPayout::Version::V1, nil]).where.not(state: ConsignmentPayout::STATE_CANCELLED) }, class_name: 'ConsignmentPayout', primary_key: 'original_item_number', foreign_key: 'reference_item_number'
  has_one  :charity_consignment_payout_record, -> { where(version_type: ConsignmentPayout::VersionType::CHARITY) }, class_name: 'ConsignmentPayout', primary_key: 'original_item_number', foreign_key: 'reference_item_number'
  has_many :price_surcharges, through: :item_price, source: :item_price_applicable_surcharges
  has_many :order_surcharges, through: :order_product, source: :order_product_surcharges
  has_many :ops_attributes
  has_many :qa_item_audits, primary_key: :item_number, foreign_key: :item_number

  has_many :item_attributes_logs, foreign_key: :item_number, primary_key: :item_number
  has_many :partner_awards

  has_one :zero_payout_item
  #### DANGER - vendor characteristics cannot be exposed to MSD! ###
  has_one :payout_estimate_v2
  has_one :reclamation

  # Partner exclusivity relations
  has_one :item_exclusive_listing

  # Reminder, order matters - pricing is dependent on merch department and Q2 discount factor
  before_validation :set_merchandising_department_if_changed
  before_validation :add_constant_discount_for_quality_downgraded_items, if: Proc.new { self.quality_code != 'Q1' }
  before_validation :ensure_price_consistency

  before_save :populate_size_label
  before_save :log_changes_if_in_debug
  before_save :log_paid_out_changes_if_in_debug
  # before_save :set_original_item_number

  # def set_original_item_number
  #   self.original_item_number ||= item_number
  # end

  after_save :determine_sourcing_policy, if: :can_change_sourcing_policy?
  after_commit :publish_state_create, on: :create # AASM never triggers an after_commit on creations
  after_commit :publish_state, on: :update, if: Proc.new { previous_changes.key?("state") } # in case the state is manually updated outside of AASM

  validates_presence_of [:details, :brand_id, :sizing_id, :product_id], if: :required_fields_for_listing?
  validate :restrict_to_final_sale

  delegate :msrp, to: :item_price, allow_nil: true
  delegate :scaling_factor, to: :brand,    prefix: true, allow_nil: true
  delegate :scaling_factor, to: :category, prefix: true, allow_nil: true
  delegate :scaling_factor, to: :size,     prefix: true, allow_nil: true
  delegate :category_groups, to: :category, allow_nil: true
  delegate :update_amount_awarded, to: :concierge_bag
  delegate :sender, to: :concierge_bag
  delegate :supplier_bonuses, to: :sender, allow_nil: true
  delegate :new_payout_rules?, to: :concierge_bag, allow_nil: true
  delegate :bag_number, to: :concierge_bag, prefix: true, allow_nil: true
  delegate :is_unidentified?, to: :concierge_bag, allow_nil: true
  delegate :calculate_upfront_payout, :calculated_upfront_payout,
    :calculate_consignment_payout, :consignment_base_price,
    :consignment_payout_estimate, :price_without_mdc,
    :apply_raas_mens_upfront_payout_partner_award, to: :item_payout
  delegate :name, to: :brand, prefix: true, allow_nil: true
  delegate :percentage_available_to_refund, to: :order_product
  delegate :refund_percentage, to: :order_product
  delegate :amount_already_refunded, to: :order_product
  delegate :percentage_refunded, to: :order_product
  delegate :adult_numeric_tier, to: :brand, allow_nil: true
  delegate :brand_tier_group, to: :brand, allow_nil: true
  delegate :adult_brand_tier_group, to: :brand, allow_nil: true
  delegate :partner_retail?, to: :item_sales_channel, allow_nil: true
  delegate :sales_channel_code, to: :item_sales_channel, allow_nil: true
  delegate :upfront_wholesale?, :wholesale?, to: :concierge_bag, allow_nil: true
  delegate :reclamation_opted_out?, to: :concierge_bag, allow_nil: true
  delegate :level_2_merch_category, to: :category, allow_nil: true

  aasm column: :state do
    state :drafted, initial: true
    state :stockroom, after_exit: :update_time_in_stockroom
    # TO DO: Add ensure_color_names_are_present callback back in to the aasm_state :listed enter once ops republishes it's color data.
    state :listed, enter: [:recalculate_price_and_final_sale, :calculate_score]
    state :ready_to_list, before_enter: :set_ready_to_list_at, enter: [:set_new_without_tags, :auto_generate_characteristics, :assign_to_q3_pail]
    state :reserved, after_exit: :update_time_in_cart
    state :requested_by_customer   # for goody box orders
    state :purchased
    state :flagged_for_relisting
    state :relisted
    state :packed, enter: [:update_reclamation, :update_upfront_offer]
    state :held_by_customer        # for goody box orders
    state :not_paid
    state :not_packed
    # packed_no_costs tells accounting that they can recognize revenue from this item but not the costs
    # since these items have been written off but then charged successfully
    state :packed_no_costs         # for goody box orders
    state :returned
    state :returned_not_paid       # for goody box orders
    state :returned_late          # for goody box orders, when user paid for item but then returned it
    state :returned_and_destroyed
    state :held
    state :lost

    state :consignment_delisted
    state :reclaimable

    state :to_be_scrapped
    state :scrapped

    state :destroyed_processing
    state :flawed_after_processed # deprecated https://thredupdev.looker.com/explore/redshift_thredup/items_v2?toggle=fil&qid=IYkYL2hycrgAWtFOXv90HN
    state :flawed_after_purchase
    state :under_review, after_exit: [:create_delayed_listing_job, :update_time_in_under_review]
    state :photo_shoot

    state :transferred
    state :to_be_transferred
    state :listed_at_store
    state :failed_to_list_at_store

    state :partner_listed # listed on another platform exclusively
    state :dropshipping_deleted # item was deleted by owner. Terminal state

    ## !!
    # If you add a new event here, don't forget the after_commit callback to publish the new state to Ops
    ## !!
    event :list, after: [:log_state, :add_to_item_pail, :update_reclaim_fee], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :reserved, to: :lost, guard: :misplaced?
      transitions from: :reserved, to: :reclaimable, guard: :should_release_to_reclaimable?, after: :unset_reclaim
      transitions from: :reserved, to: :consignment_delisted, guard: :release_to_delisted?
      transitions from: [:ready_to_list, :reserved, :photo_shoot, :consignment_delisted, :partner_listed], to: :listed, guard: :should_be_listed?
      transitions from: :stockroom, to: :listed, guard: :should_transition_from_stockroom_to_listed? # Transition for SKU items to move directly from stockroom to listed
      transitions from: :reserved, to: :stockroom, guard: :has_sku?
      transitions from: :reclaimable, to: :listed, guard: :can_list_reclaimable?
    end

    event :partner_list, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :ready_to_list, to: :partner_listed, guard: :partner_listable_guard
    end

    event :mark_ready_to_list, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :stockroom, to: :stockroom, guard: :list_in_stockroom_guard
      transitions from: [:stockroom], to: :ready_to_list, guard: :listable_guard
    end

    event :postprocess_dc_item, after: [:unlist_from_review, :set_merchandising_department_and_save, :create_exclusive_listings, :log_state, :backfill_stockroom_time], after_commit: [:publish_state, :publish_item_feed_event, :nullify_listed_price] do
      transitions from: :stockroom, to: :listed, guard: :wholesale_listable
      transitions from: :stockroom, to: :stockroom, guard: :wholesale_stockroomable
      transitions from: :stockroom, to: :ready_to_list, guard: :stockroom_guards_cleared?
      transitions from: :under_review, to: :stockroom
      transitions from: :drafted, to: :stockroom, guard: :list_in_stockroom_guard # sanity check for next line
      transitions from: :drafted, to: :ready_to_list, guard: :requires_no_stockroom_wait?
      transitions from: [:drafted, :stockroom], to: :stockroom
    end

    event :stockroom_relisted_item, after: :log_state, after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :drafted, to: :stockroom
    end

    event :photo_shoot, after: [:log_state, :relist_after_7_days], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :listed, to: :photo_shoot
    end

    event :consignment_delist, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: [:listed, :reclaimable], to: :consignment_delisted, guard: :consignment_delist_allowed?
    end

    event :make_reclaimable, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :listed, to: :reclaimable
    end

    event :queue_for_scrap, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event, :release_another] do
      transitions from: [:listed], to: :to_be_scrapped
      transitions from: [:stockroom], to: :to_be_scrapped, guard: :paid_out?
    end

    event :scrap, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: [:to_be_scrapped, :scrapped], to: :scrapped
    end

    event :reserve, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event, :release_another] do
      transitions from: [:listed, :partner_listed, :reclaimable], to: :reserved
      transitions from: :reserved, to: :reserved, guard: :reclaim_on_release?
      transitions from: :stockroom, to: :reserved, guard: :remade_reservable_guard
    end

    event :requested_by_customer, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :reserved, to: :requested_by_customer
      transitions from: :listed, to: :requested_by_customer
    end

    event :flag_for_review, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: %i[listed stockroom ready_to_list partner_listed], to: :under_review
    end

    event :release, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event, :mark_repricing_as_completed, :relist_under_superbag_if_raas_consignment_window_completed] do
      transitions from: :reserved, to: :lost, guard: :misplaced?
      transitions from: :reserved, to: :under_review, guard: :flagged?
      transitions from: :reserved, to: :reclaimable, guard: :should_release_to_reclaimable?, after: :unset_reclaim
      transitions from: :reserved, to: :partner_listed, guard: :exclusive?
      transitions from: :reserved, to: :consignment_delisted, guard: :release_to_delisted?
      transitions from: :reserved, to: :listed, guard: :should_release_to_listed?
      transitions from: :reserved, to: :stockroom
    end

    event :purchase,
      after: [:update_price_on_purchase, :log_state, :freeze_price, :set_reclaim_item_product],
      after_commit: [:publish_state, :record_payout, :publish_item_feed_event, :mark_repricing_as_failed, :enqueue_datadog_report] do

      transitions from: [:reserved, :listed_at_store, :partner_listed], to: :purchased
    end

    event :mark_not_packed, after: [:log_state, :create_partner_consignment_award, :set_consignment_payout], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :packed, to: :not_packed
      transitions from: :purchased, to: :not_packed
      transitions from: :requested_by_customer, to: :not_packed, guard: :item_box_item?
    end

    event :hold, after: :log_state, after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :drafted, to: :held
    end

    event :remove_hold, after: :log_state, after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :held, to: :drafted
    end

    event :pack, after: [:log_state, :create_partner_consignment_award, :set_consignment_payout], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :purchased, to: :packed
      transitions from: :requested_by_customer, to: :held_by_customer, guard: :item_box_item?
    end

    event :item_box_purchase, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :held_by_customer, to: :packed, guard: :item_box_item?
      transitions from: :not_paid, to: :packed_no_costs, guard: :order_in_terminal_state?
    end

    event :destroy_processing, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event, :create_payout_for_returned_item, :nullify_listed_price] do # dont ever use this. Only meant for ops
      transitions from: [:drafted, :stockroom, :held, :under_review], to: :destroyed_processing
    end

    event :unlist, after: [:set_reitemization_listing_hold, :log_state], after_commit: [:publish_state, :publish_item_feed_event] do # in case ops wants to reitemize garments already controlled by web
      transitions from: [:stockroom, :ready_to_list, :listed, :partner_listed], to: :stockroom, guard: :listing_hold?
      transitions from: [:stockroom, :ready_to_list, :listed, :partner_listed], to: :drafted
    end

    event :mark_lost, after: :log_state, after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: [
        :listed,
        :stockroom,
        :ready_to_list,
        :drafted,
        :under_review,
        :to_be_transferred,
        :to_be_scrapped
      ],
                  to: :lost
    end

    event :not_paid, after: [:log_state], after_commit: :publish_state do
      transitions from: [:held_by_customer, :not_paid], to: :not_paid
    end

    event :receive_return_and_destroy, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: [:packed, :flawed_after_purchase, :held_by_customer, :not_paid, :returned, :returned_and_destroyed, :returned_not_paid], to: :returned_and_destroyed
    end

    event :receive_return, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :not_paid, to: :returned_not_paid
      transitions from: [:held_by_customer, :returned_not_paid, :not_paid], to: :returned_not_paid, guard: :item_box_item?
      transitions from: :packed, to: :returned_late, guard: :item_box_item?
      transitions from: [:packed, :flawed_after_purchase, :returned], to: :returned
      transitions from: :returned_and_destroyed, to: :returned_and_destroyed
    end

    event :mark_as_returned_and_destroyed, after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: [:packed, :flawed_after_purchase, :returned], to: :returned_and_destroyed
    end

    event :flag_for_relist, after_commit: [:publish_state] do
      transitions from: :listed, to: :flagged_for_relisting
    end

    event :move_to_relisted, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :flagged_for_relisting, to: :relisted
      transitions from: :not_paid, to: :not_paid
      transitions from: :purchased, to: :relisted
      transitions from: :requested_by_customer, to: :relisted
    end

    event :mark_to_be_transferred, before: [:set_price_for_stockroom], after: [:log_state, :buyout_consignment_item_for_store, :release_another], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: [:drafted, :listed, :listed_at_store], to: :to_be_transferred
      transitions from: [:stockroom], to: :to_be_transferred, guard: :stockroom_to_be_transferred_guard?
    end

    event :failed_to_list_at_store, after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: [:listed, :to_be_transferred], to: :failed_to_list_at_store
    end

    event :list_at_store, after: [:ensure_stockroom_at, :clear_stockroom_reason, :log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: [:drafted, :stockroom, :failed_to_list_at_store], to: :listed_at_store
    end

    event :transfer, after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: [:to_be_transferred], to: :transferred
    end

    event :move_to_stockroom, after: [:log_state], after_commit: [:publish_state, :publish_item_feed_event] do
      transitions from: :listed, to: :stockroom
    end

    event :dropshipping_delete, after: [:log_state], after_commit: [:publish_item_feed_event] do
      transitions from: [:listed, :reserved, :stockroom, :ready_to_list], to: :dropshipping_deleted, guard: :dropshipping?
    end
  end

  LISTED_ALL = ['listed', 'reserved', 'purchased', 'packed', 'sent', 'received', 'not_packed']
  INDEXED_ALL = %w[ready_to_list listed reserved]
  ITEMIZED_ALL = ['stockroom', 'ready_to_list'] | LISTED_ALL
  PURCHASED_ALL = ['purchased', 'packed', 'sent', 'received']
  ALWAYS_UPFRONT = ['destroyed_processing', 'flawed_after_processed', 'flawed_after_purchase', 'scrapped', 'to_be_scrapped']
  PAID_OUT_ALL = [
    'listed', 'reserved', 'purchased',
    'packed', 'sent', 'received', 'stockroom', "flawed_after_purchase", "not_packed",
    'under_review', 'ready_to_list', 'rejected_price_floor', 'to_be_scrapped',
    'scrapped', 'returned', 'relisted', 'lost', 'to_be_transferred',
    'transferred']
  PAID_OUT_AND_RECLAIMABLE = [
    'listed', 'reserved', 'purchased',
    'packed', 'sent', 'received', 'stockroom', "flawed_after_purchase", "not_packed",
    'under_review', 'ready_to_list', 'rejected_price_floor', 'to_be_scrapped',
    'scrapped', 'returned', 'relisted', 'missing', 'reclaimable']
  CAN_BE_MARKED_AS_RECLAIMABLE_BY_CS = %w[listed reserved]
  UNAWARDABLE = ['drafted', 'soft_destroyed', 'held']
  AVAILABLE = %w(listed reclaimable)

  DISPLAY_FOR_SUPPLIER = %w(
    photo_shoot
    under_review
    lost
    to_be_transferred
    transferred
    requested_by_customer
    held_by_customer
    returned_not_paid
    not_paid
  )
  GOODY_BOX_STATES = %w(requested_by_customer held_by_customer returned_not_paid)

  UPFRONT_OFFER_ACCEPT_STATES = %w(stockroom listed ready_to_list reserved requested_by_customer under_review held_by_customer)
  RETAIL_PARTNER_PAYOUTABLE_STATES = %w(to_be_transferred taged packed)
  MAX_COMMISSION_RATE = 1

  UPFRONT_PAYOUT_POLICY = 'upfront'
  CONSIGNMENT_PAYOUT_POLICY = 'consignment'

  REITEMIZATION_LISTING_HOLD = 'hold_for_reitemization'

  CHILDREN_GENDERS = %w( boys girls unisex)
  ADULT_GENDERS = %w(teen-girls women men)
  ALL_ITEM_GENDERS = ["girls","boys","women"]
  TUP_BABY_SIZES = [
    1, 2, 4, 6, 7, 8, 10, 11, 12, 14, 15, 16, 73, 74, 76, 78, 79, 80, 82, 83, 84, 86, 87, 88,
    348, 349, 352, 354, 357, 360, 362, 365, 366, 369, 371, 374, 375, 912, 913, 914, 915, 916, 917, 918, 919, 920, 922, 923]

  PARTIAL_REFUND_PERCENTAGE = 25
  QUALITY_REFUND_PERCENTAGE = 15
  DESCRIPTION_REFUND_PERCENTAGE = 15
  Q2_REFUND_PERCENTAGE      = 25
  Q3_REFUND_PERCENTAGE      = 40
  FULL_REFUND_PERCENTAGE    = 100

  FIELDS_AFFECTING_PRICE = %w(merchandising_department new_with_tags adjustable_waist gender brand_id category_id sizing_id summer winter fall spring set_of_items quality_code)
  VARIABLE_PRICE_STATES = %w(drafted listed stockroom under_review)

  FALL_CATEGORIES_IDS = [
    329, 331, 332, 333, 334, 335, 336, 337,
    338, 341, 342, 343, 356, 362, 373, 378, 384, 400,
    405, 736, 737, 738, 778, 795, 799, 812,
    813, 814, 847, 983, 984, 985, 986, 987, 988]
  FALL_BRANDS_IDS = [
    287, 1073, 2575, 8687, 9293, 12884, 16768, 18980, 23552, 35694,
    36533, 38100, 39462, 39895, 41273, 42358, 42720, 44771]

  scope :listed_all, -> { where(state: LISTED_ALL) }
  scope :itemized_all, -> { where(state: ITEMIZED_ALL) }
  scope :purchased_all, -> { where(state: PURCHASED_ALL) }
  scope :paid_out_all, -> { where(state: PAID_OUT_ALL) }
  scope :select_cache_info, -> { select("items.id, items.updated_at, items.created_at") }
  scope :awarded_all, -> { where("(items.original_item_number is null OR items.item_number = original_item_number) and state NOT IN (?)", UNAWARDABLE) }

  scope :for_sale, -> { where(state: ["listed","reserved"]) }
  scope :with_sku, -> (s) { where(sku: s) }
  scope :consignment, -> { where(payout_policy: "consignment") }
  scope :upfront, -> { where(payout_policy: "upfront") }
  scope :paid_out, -> { where(paid_out: true) }
  scope :unpaid, -> { where(paid_out: false) }

  scope :top_brand_items, -> { where(brand_id:  Rules::TopBrands::TOP_BRANDS_IDS) }
  scope :good_condition, -> { where("quality_code = 'Q1' OR new_with_tags = true") }
  # Scope for selecting items by actual listed (listed_on_site_since) date,
  # which does not include time item was in the cart
  #
  # Examples:
  # .actual_listed(:after, '2017-12-22 12:00:00')
  # .actual_listed(:between, '2017-12-22 12:00:00', '2017-12-28 20:00:00')
  scope :actual_listed, -> (action, date1, date2 = nil) do
    scope = readonly(false).with_originals.
      joins("LEFT JOIN item_time_spent_by_state ON original_items_items.id = item_time_spent_by_state.item_id")

    field = "COALESCE(item_time_spent_by_state.adjusted_listed_at, original_items_items.listed_at)"

    case action
    when :after
      scope.where("#{field} > ?", date1)
    when :before
      scope.where("#{field} < ?", date1)
    when :between
      scope.where("#{field} BETWEEN ? AND ?", date1, date2)
    end
  end

  scope :with_originals, -> { joins("INNER JOIN items original_items_items ON original_items_items.item_number = items.original_item_number") }

  scope :random, -> { order('RAND()') }

  #singular filter scopes, typically filtering by name or label
  scope :size_filter, -> (size_type, size_value) { joins(:sizing).where("sizings.type_name = ? AND sizings.measurement_1 = ?", size_type, size_value) }
  scope :brand_filter, -> (brand) { joins(:brand).where("brands.name = ?", brand) }
  scope :quality_code_filter, -> { where("items.quality_code is NULL") }

  #collection filters, filtered by ids
  scope :with_brands, -> (brand_ids) { where("brand_id in (?)", brand_ids) }
  scope :with_categories, -> (category_ids) { where("category_id in (?)", category_ids)}
  scope :with_sizes, -> (size_ids) { where("sizing_id in (?)", size_ids) }
  scope :filter_min_price, -> (min_price) { joins(:product).where("products.base_price >= ?", min_price) }

  # brand tiers
  BRAND_TIER_FIELD_SQL = "IF(items.gender='women',COALESCE(brands.adult_brand_tier_id,brands.brand_tier_id),brands.brand_tier_id)"
  JOIN_BRAND_TIERS_SQL = "JOIN brands ON brands.id = items.brand_id JOIN tiers ON tiers.id=#{BRAND_TIER_FIELD_SQL}"
  scope :join_brand_tiers, -> { joins(JOIN_BRAND_TIERS_SQL) }

  scope :filter_min_budget, -> (min_tier) { join_brand_tiers.where("tiers.scaling_factor >= ?", min_tier) }
  scope :filter_budget_with_range, -> (min_tier, max_tier) { join_brand_tiers.where("tiers.scaling_factor between (?) and (?)", min_tier, max_tier )}

  scope :filter_with_range, -> (min_price, max_price) { joins(:product).where("products.base_price between (?) and (?)", min_price, max_price )}

  scope :from_bags, -> (bag_ids) { where('concierge_bag_id in (?)', bag_ids) }
  scope :filter_item_score, -> (min_score) { joins(:item_scores).where("item_scores.score >= ?", min_score) }

  scope :children, -> { where(gender: CHILDREN_GENDERS) }
  scope :adults, -> { where(gender: ADULT_GENDERS) }

  scope :supplier_states, -> (*additional_states) {
    where(state: (Item::PAID_OUT_AND_RECLAIMABLE | Item::DISPLAY_FOR_SUPPLIER | additional_states).flatten).
      where("((items.original_item_number is null OR items.item_number = items.original_item_number) AND items.payout_policy = 'upfront') OR (items.payout_policy='consignment')")
  }

  scope :for_bag_details, -> (*additional_states) { supplier_states(*additional_states).order(:gender) }

  # Select not paid out and not reclaimed items
  scope :active_items, -> {
    for_bag_details.
      joins(:concierge_bag).
      joins('LEFT JOIN order_products ai_order_products on ai_order_products.item_id = items.id').
      joins('LEFT JOIN orders ai_orders on ai_order_products.order_id = ai_orders.id').
      where(items: { paid_out: [false, nil] }).
      where("items.payout_policy != 'consignment' OR items.purchased_at IS NULL OR ai_orders.user_id != concierge_bags.user_id").
      where("items.state NOT IN ('relisted', 'returned', 'returned_not_paid')")
  }

  scope :not_from_superuser, -> { joins(:concierge_bag).where.not(concierge_bags: { user_id: User::SUPER_USER_ID }) }
  scope :not_dropshipping, -> { where(dropshipping_warehouse_id: nil) }

  def self.filter_duplicated_relisted(items)
    grouped_items = items.group_by do |i|
      # If item does not have original item number we want
      # to put it to the same group with its derivatives.
      # For this we use item number as a group key
      i.original_item_number || i.item_number
    end

    grouped_items.values.map do |derivatives|
      # Take paid out or the latest created item from the list
      derivatives.max_by { |i| [i.paid_out ? 1 : 0, i.created_at] }
    end
  end

  def self.maintain_order_sql(item_numbers)
    return if item_numbers.blank?
    
    Arel.sql("FIELD(items.item_number, #{item_numbers.join(',')})")
  end

  def item_update_log
    @item_update_log ||= CustomLogger.new("item_changes_before_save.log",{ include_backtrace: true })
  end

  def log_changes_if_in_debug
    begin
      log = $redis.get("log_model_changes:Item")
    rescue Redis::CannotConnectError, Redis::TimeoutError
      Rollbar.error("Unable to connect to redis")
      log = false
    end

    return unless log

    c = self.changes
    return if c.keys == ["view_count"] || c.empty?

    payload = {
      item_number: self.item_number,
      model: "Item",
      changes: c
    }
    item_update_log.info(payload)
  end

  def log_paid_out_changes_if_in_debug
    begin
      log = $redis.get("log_paid_out_changes:Item")
    rescue Redis::CannotConnectError, Redis::TimeoutError
      Rollbar.error("Unable to connect to redis")
      log = false
    end

    return unless log

    chg =  self.changes
    return if chg["paid_out"]&.last != true && !chg.key?("commission_rate")

    trace_depth = 10
    trace_lines = caller.reject { |line| line =~ /gems/ }.first(trace_depth)

    record_updates = {
      changes: chg,
      attributes: {
        item: self.attributes,
        item_price: self.item_price
      }
    }

    payload = {
      item_number: self.item_number,
      record_updates: record_updates.to_json,
      backtrace: trace_lines,
      timestamp: Time.now
    }

    ItemAttributesLog.create(payload)
  end

  def full_commission?
    commission_rate == MAX_COMMISSION_RATE
  end

  def update_upfront_offer
    return if upfront_offer.nil? || !upfront_offer.proposed?

    upfront_offer.mark_item_sold!
  end

  def update_reclamation
    return unless reclamation&.may_reclaim?

    reclamation.reclaim! if order_product&.product&.reclaimed?
  end

  def listed_at
    return (self[:listed_at] || @listed_at) if (self[:listed_at] || @listed_at)

    lookup_first_listed_at_in_history
  end

  def ops_state
    @ops_attr ||= ops_attributes.ops_state.first
    @ops_attr&.value
  end

  def ops_state=(value)
    @ops_attr ||= ops_attributes.first_or_initialize(name: 'ops_state')
    @ops_attr.value = value
  end

  def lookup_first_listed_at_in_history
    self.history.where(state: "listed").select("MIN(created_at) AS listed_at").first.try(:listed_at)
  end

  def publish_state
    payload = { event: "ItemUpdate", source: "Web", data: { item_number: item_number, state: state } }.to_json
    $publisher_ops_direct.publish(payload, to_queue: rabbit_queue('sneakers_general'))
  rescue => e
    if Rails.env.development?
      puts "ItemUpdate state publishing error"
      puts e.inspect
      puts e.backtrace
    else
      Rollbar.error(e, "ItemUpdate state publishing error for item number: #{item_number}")
    end
  end
  alias publish_state_create publish_state

  def relist_after_7_days
    Async::Sidekiq::ListItem.perform_in(7.days, id)
  end

  def calculate_final_sale
    FinalSaleRule.eligible_for_final_sale?(self)
  end

  def assign_final_sale
    self.final_sale = calculate_final_sale
  end

  def update_final_sale!
    update_attributes(final_sale: calculate_final_sale)
  end

  def final_sale?
    final_sale || false
  end

  def add_to_item_pail
    if self.state == "listed" && self.brand && !self.juniors?
      BrandPail.where(brand_id: self.brand.id).each do |bb|
        if self.can_go_in_pail?(bb.pail)
          ItemPail.create(item_id: self.id, pail_id: bb.pail_id)
        end
      end
    end
  end

  def publish_item_feed_event(event_source: nil)
    if self.aasm.current_event.to_s == 'list' && self.state == 'ready_to_list'
      Rollbar.debug('Invalid itemFeed event', { item_number: self.item_number, aasm_event: self.aasm.current_event, state: self.state, stacktrace: caller })
    end
    EventBus::ItemFeedPublisher.new.publish_item(self, event_source: event_source)
  rescue => e
    Rollbar.error(e, 'publish_item_feed_event exception raised')
  end

  def publish_item_price_event(old_price = nil)
    EventBus::ItemPricePublisher.new(self, old_price).publish_event
  rescue => e
    Rollbar.error(e, 'publish_price_event exception raised')
  end

  def create_payout_for_returned_item
    return if original_item? || paid_out? || upfront?
    CreatePayoutForReturnedItem.new(self).call
  end

  def enqueue_datadog_report
    Async::Sidekiq::Item::ReportReclamation.perform_async(self.id)
  end

  def feed_overlay_cache_key
    "feed_overlay_#{self.item_number}_#{self.product.base_price}"
  end

  def relist_under_superbag_if_raas_consignment_window_completed
    return unless is_raas_and_consignment_listing_window_has_completed?

    DestroyListing.evaluate_for_raas_consignment_window(self)
  rescue => e
    Rollbar.error(e, 'error on relist_under_superbag_if_raas_consignment_window_completed')
  end

  def photo_number
    Rails.cache.fetch("item_primary_photo_number_#{self.item_number}") do
      self.photos.where(photo_type: 'garment', photo_subtype: "front").first.photo_number
    end
  end

  def can_go_in_pail?(pail)
    return false unless pail

    (ItemPail.where(item_id: self.id, pail_id: pail.id).count == 0) && (pail.gender == self.gender)
  end

  def indexed_attributes_were_changed
    # this is not necessary a complete list of indexed fields, it is just the fields for which we want to index the item again if they are changed.
    indexed_fields = %w{category_id brand_id size_id gender new_with_tags adjustable_waist sizing_id quality_code product_id color_names}
    self.changes.keys.any? { |field| field.in?(indexed_fields) }
  end

  def self.gender_filter(gender)
    if gender.nil? || gender == '-'
      children
    elsif gender == "women"
      where("items.gender = ?", gender)
    else
      where("items.gender = ? or items.gender = ?", gender, 'unisex')
    end
  end

  def require_reason_or_raise_exception
    self.reason.present? || (raise ThredupError::LogicError.new "Reason field is not populated")
  end

  def available?
    self.state.in?(Item::AVAILABLE)
  end

  def in_category?(category_name)
    Rails.cache.fetch("item_category_membership_#{category_name.downcase}_#{self.item_number}") do
      self.category_groups.any? do |cat_group|
        (cat_group.try(:parent).try(:name).try(:downcase).try(:include?, category_name.downcase)) || cat_group.name.try(:downcase).try(:include?, category_name.downcase)
      end
    end
  end

  def belongs_to_category_by_id?(cat_id)
    self.category_id == cat_id || (self.category.parent_category.present? && self.category.parent_category.id == cat_id)
  end

  def sender_origin
    return unless self.concierge_bag && self.sender

    address = self.concierge_bag.user.shipping_address
    return unless address

    address.city.titleize + ", " + address.state.upcase
  end

  def q2?
    self.quality_code && self.quality_code.downcase == "q2"
  end

  def q3?
    self.quality_code && self.quality_code.downcase == "q3"
  end

  def quality_reasons_formatted
    flaw_codes = self.quality_reasons
    if flaw_codes && flaw_codes.length > 0
      reasons = flaw_codes.downcase.gsub(" ","_").split("|").map { |flaw_code| I18n.t("condition.q2")[flaw_code.to_sym] }
      reasons.join(", ")
    end
  end

  def add_constant_discount_for_quality_downgraded_items
    if quality_code == 'Q2'
      self.quality_discount_rate = Q2_DISCOUNT_RATE
    elsif quality_code == 'Q3'
      self.quality_discount_rate = Q3_DISCOUNT_RATE
    end
  end

  def self.filter_color(hexes)
    filter = []

    hexes = [hexes] if hexes.is_a?(String)

    hexes.each do |hex|
      search = ColorSearch.new(hex)
      filter1 = search.filter(1)
      filter2 = search.filter(2)
      filter << "(#{filter1}) or (#{filter2})"
    end

    where(filter.join(' or '))
  end

  # items greater than $40 may not be applicable for certain promo discounts
  def self.maximum_discount_criteria
    4000
  end

  def get_info_for_queued_message
    item = self.pre_process
    { 'id' => item.id,
      'brand' => item.brand.name.titleize,
      'category' => item.category.siv_display_name.titleize,
      'original_price' => item.original_price,
      'savings' => item.savings,
      'price' => item.price,
      'department' => item.department,
      'size' => item.size_label,
      'url' => item.photo_urls[:primary][:medium_url]
    }
  end

  def self.ids_to_item_numbers(ids)
    return [] unless ids.is_a?(Array)

    self.where(id: ids).pluck(:item_number)
  end

  def required_fields_for_listing?
    ITEMIZED_ALL.include? self.state
  end

  def relist! # Only to be used for relisting items in cancelled orders.
    OpsApi.relist_canceled_order_item(item_number)
  end

  def total_awarded
    return self.upfront_payout
  end

  def formatted_total_awarded
    return currencify(total_awarded / 100.00, { currency_symbol: "$ " })
  end

  def display_upfront_payout
    if upfront?
      supplier_bag_item&.actual_payout
    end
  end

  def display_consignment_payout
    if consignment?
      supplier_bag_item&.actual_payout || consignment_payout
    end
  end

  def unlist_from_review
    item_flags.for_review.active.map(&:resolve!)
  end

  def unset_reclaim
    item_flags.for_reclaim.active.map(&:resolve!)
  end

  def sale
    false
  end

  def on_sale
    self.sale && self.first_listing_price.present? && (self.first_listing_price > self.product.base_price)
  end

  def clearance?
    # Must be 90 days or older on the site
    return false unless self.listed_at && (self.listed_at <= 90.days.ago)

    # Must have been marked down by PLR or clearance
    markdown = item_price_markdowns.completed_repricing.last
    markdown.present? && (markdown.markdown? || markdown.reprice_markdown?)
  end

  def current_price
    self.product.try(:base_price)
  end

  def stockroom_guards_cleared?
    (listable? && !belongs_in_stockroom?)
  end

  def wholesale_listable
    (upfront_wholesale? && listable? && !belongs_in_stockroom?)
  end

  def wholesale_stockroomable
    (upfront_wholesale? && listable? && belongs_in_stockroom?)
  end

  def list_in_stockroom_guard
    (listable? && belongs_in_stockroom?)
  end

  def stockroom_to_be_transferred_guard?
    !listing_hold? &&
    listable? &&
    !warehouse_listing_disabled? &&
    !bag_items_not_listable? &&
    !consignment? &&
    !photo_missing? &&
    !sourcing_policy_stockroom_hold? &&
    wholesale_data_valid_for_stockroom_to_be_transferred?
  end

  def set_price_for_stockroom
    return unless stockroom?

    recalculate_price_and_final_sale
    calculate_score
  end

  def bag_processed_guard
    self.concierge_bag.try(:processed?) || self.in_superbag?
  end

  def bag_bought_out_guard
    self.concierge_bag && self.concierge_bag.bought_out? && self.listable?
  end

  def relisted_item_guard
    derivative_item?
  end

  # The purpose of this is simply for readability of the method belongs_in_stockroom?
  def bag_items_not_listable?
    return false if consignment_relistable?

    !bag_items_listable?
  end

  def bag_items_listable?
    if concierge_bag
      concierge_bag.items_listable?
    else
      # Some old items have concierge bag id = 0 from before we tracked items to bags ine early 2012.
      true
    end
  end

  def consignment_relistable?
    upfront_payout > 0 && paid_out? && payout_policy == "consignment" && in_superbag? && derivative_item?
  end

  def no_price_assigned?
    item_price.blank? || item_price.try(:price).blank?
  end

  def no_price_assigned_by_price_tagger?
    return @not_priced_by_tagger if defined?(@not_priced_by_tagger)
    return false if remade? || item_price&.is_admin_adjusted

    @not_priced_by_tagger = PricingService::Client.should_belong_to_stockroom?(item_number)
    Item.datadog_client.increment("item.not_priced_by_price_tagger", tags: ["outcome:#{@not_priced_by_tagger}", "state:#{state}"])

    @not_priced_by_tagger
  rescue StandardError => e
    Rollbar.error(e, "Unexpected error during no_price_assigned_by_price_tagger check")
    false
  end

  def belongs_in_stockroom?
    # NOTE - This method is used to tell if an item should be held back in the stockroom.
    # If the item fails all these checks and is listable? then list! will send it straight to ready_to_list
    # If any one of the checks is true, then the item will remain in stockroom
    stockroom_checks = %W{
      bag_items_not_listable?
      sku_item_already_listed?
      photo_missing?
      sourcing_policy_stockroom_hold?
      warehouse_listing_disabled?
      listing_hold?
      sku_style_inactive?
      no_price_assigned?
      no_price_assigned_by_price_tagger?
    }

    stockroom_checks.each do |m|
      belongs = !!self.send(m)
      if belongs
        self.stockroom_reason = m.gsub("?","")
        return belongs
      end
    end
    self.stockroom_reason = nil
    false
  end

  def jewelry?
    category_id.in?(Category::JEWELRY_IDS)
  end

  def puzzle?
    category_id == Category::PUZZLES_ID
  end

  def warehouse_listing_disabled?
    !!warehouse_item_setting.try(:listing_disabled?)
  end

  def handbag?
    Rails.cache.fetch("is_handbag_#{category_id}") do
      category_groups && category_groups.any? { |cat_group| ( cat_group.parent && cat_group.parent.name == "Handbags" ) || cat_group.name == "Handbags" }
    end
  end

  def plus_size?
    return false if sizing_modifier_id.nil?

    sizing_modifier_id == SizingModifier::plus_modifier_id
  end

  def maternity?
    sizing_modifier_id == SizingModifier::MATERNITY_SIZING_MODIFIER_ID || brand.try(:is_maternity)
  end

  def shoe?
    if category_groups
      category_groups.any? { |cat_group| ( cat_group.parent && cat_group.parent.name == "Shoes" ) || cat_group.name == "Shoes" }
    else
      false
    end
  end

  def necklace?
    category_id.in?(Category::NECKLACE_IDS)
  end

  def denim?
    category_id.in?(Category::DENIM_IDS)
  end

  def earring?
    category_id.in?(Category::EARRING_IDS)
  end

  def shoe_tag?
    shoe? || necklace? || earring?
  end

  def from_women_department?
    merchandising_department == 'women'
  end

  def from_shoe_department?
    merchandising_department == 'women shoes'
  end

  def thredupx?
    thredupx_status == "active"
  end

  def purchased_on_listing_partner?
    return false unless (order = order_product.try(:order))
    order.after_purchase? && order.from_listing_partner?
  end

  def front_photo
    self.photos.where(photo_subtype: "front").first
  end

  def sku_item_already_listed?
    (has_sku? && !open_sku_slot_exists?)
  end

  def full_upfront_partner?
    concierge_bag.full_upfront_partner?
  end

  def allowed_to_accept_an_offer?
    UPFRONT_OFFER_ACCEPT_STATES.include?(state) &&
      !paid_out
  end

  def from_stichfix_wholesale?
    # Reusable restriction on listing items from stichfix_wholesale.
    self.concierge_bag.present? && self.concierge_bag.partner_id == 23 # 23 is the stichfix wholesale id
  end

  def has_sku?
    self.sku.present?
  end

  def requires_no_stockroom_wait?
    (bag_bought_out_guard || bag_processed_guard || relisted_item_guard) && listable_guard
  end

  def partner_listable_guard
    listable? && listing_hold.blank?
  end

  def listable_guard
    listable? && (!has_sku? || open_sku_slot_exists?) && listing_hold.blank?
  end

  def should_be_listed?
    listable_guard && !exclusive?
  end

  def should_transition_from_stockroom_to_listed?
    listable_guard && stockroom_guards_cleared?
  end

  def formerly_stockroomed_guard
    history.pluck(:state).include? 'stockroom'
  end

  def sku_listable_guard
    has_sku? && listable_guard
  end

  def release_another
    Async::Sidekiq::Item::ReleaseAnother.perform_async(id)
  end

  # Are there items with the same sku + warehouse + condition (for Remade) in listed state
  def open_sku_slot_exists?
    return true if partner&.rtr_revive?
    !same_sku_and_warehouse_items(:listed).exists?
  end

  def same_sku_and_warehouse_items(state = nil)
    items = self.class.with_sku(sku).where(warehouse_id: warehouse_id, listing_hold: nil)

    if state.present?
      items = items.where(state: state)
    end

    if remade?
      search_clause = SkuStyle::Conditions::MAPPING[remade_condition]
      items = items.where(search_clause).joins(:concierge_bag).where('concierge_bags.state = "processed"').readonly(false)
    end

    # Ensure only RTR/Revive items that have finished processing in ops are returned
    if self.partner&.rtr_revive?
      items = items.joins(:concierge_bag).where('concierge_bags.state = "processed"').readonly(false)
    end

    items
  end

  def set_merchandising_department_if_changed
    return if drafted? && stockroom_at.nil?

    if (!(changed & %w{gender brand_id category_id sizing_modifier_id sizing_id thredupx_status}).empty? || self.merchandising_department.blank?)
      set_merchandising_department
    end
  end

  def set_merchandising_department_and_save
    set_merchandising_department
    if changed.count > 0
      save
    end
  end

  def set_merchandising_department
    return unless can_calculate_merchandising_department?

    self.merchandising_department = calculate_merchandising_department
  end

  def create_exclusive_listings
    return if partner.blank? || partner.business.blank? || !partner.business.has_exclusive_rules?
    return unless partner.business.business_exclusive_rule.match_criteria?(self)
    ItemExclusiveListing.where(
      item_id: id,
      business_id: partner.business.id
    ).first_or_create! do |iel|
      iel.active = true
    end
  end

  def can_calculate_merchandising_department?
    !!(gender && brand_id && category_id && sizing_id)
  end

  def merch_dept_log
    @merch_dept_log ||= CustomLogger.new("merch_dept_calcs.log",{ include_backtrace: true })
  end

  def calculate_merchandising_department
    detail = {
      item_id: id,
      item_number: item_number,
      gender: gender,
      category_id: category_id,
      handbag: handbag?,
      maternity: maternity?,
      sizing_id: sizing_id,
      brand_id: brand_id,
      brand_is_junior: brand.try(:is_junior),
      thredupx_status: thredupx_status
    }
    merch_dept_log.info(detail)

    case gender
    when "girls"
      "girls"
    when "unisex","boys"
      "boys"
    when "women","teen-girls","juniors"
      if self.category_id.in?(Category::WOMENS_SHOES_CATEGORY_IDS)
        "women shoes"
      elsif self.handbag?
        "handbags"
      elsif self.maternity?
        "maternity"
      elsif sizing_is_plus_merch? || brand_is_plus_merch?
        "plus"
      elsif sizing_id.in?(Sizing::JUNIORS_ONLY_SIZING_IDS) || gender.in?(["teen-girls","juniors"]) || brand.try(:is_junior)
        "juniors"
      elsif thredupx_status == "active"
        "X"
      else
        "women"
      end
    else
      gender
    end
  end

  def department_id
    cached_departments = Department.to_hash
    if self.merchandising_department == "juniors"
      return cached_departments['juniors']
    else
      return cached_departments[self.gender]
    end
  end

  def display_department
    self.department.capitalize
  end

  def kids_item?
    self.gender.in? ["boys", "girls", "unisex"]
  end

  def department
    self.merchandising_department == "juniors" ? "Juniors" : self.gender.try(:humanize)
  end

  def juniors?
    self.gender == "women" && (self.sizing_is_juniors_only? || brand_is_junior?)
  end

  def womens?
    self.gender == "women"
  end

  def mens?
    self.gender == "men"
  end

  def sizing_is_juniors_only?
    Sizing.sizing_id_is_juniors_only?(sizing_id)
  end

  def brand_is_junior?
    if self.junior_brand.nil?
      self.brand.try(:is_junior)
    else
      self.junior_brand.to_i == 1
    end
  end

  def brand_scaling_factor
    return nil if brand.nil?

    if gender == "women"
      t = brand.adult_brand_tier || brand.brand_tier
      t.try(:scaling_factor)
    else
      brand.brand_tier.try(:scaling_factor)
    end
  end

  def calculate_sizing_modifier_id
    if gender == "women"
      return SizingModifier.maternity_modifier_id if Brand.brand_id_is_maternity?(brand_id)
      return SizingModifier.plus_modifier_id if sizing_is_plus_merch?
    end

    sizing_modifier_id
  end

  def womens_plus?
    self.gender == "women" && (self.sizing_modifier_id == SizingModifier.plus_modifier_id || sizing_is_plus_merch?)
  end

  def sizing_is_plus_merch?
    Sizing.is_plus_merch?(sizing_id)
  end

  def brand_is_plus_merch?
    brand_id.in?(Brand::PLUS_ONLY_IDS)
  end

  # womens item AND (sizing_modifier is maternity OR brand is maternity)
  def maternity_item?
    self.gender == "women" && ((self.sizing_modifier && self.sizing_modifier.id == SizingModifier.maternity_modifier_id) || self.brand.try(:is_maternity))
  end

  def normalized_gender
    case self.gender
    when "boys","unisex"
      "boys"
    when "girls"
      "girls"
    when "women","teen-girls"
      "women"
    end
  end

  def listable?

    # Items with sku's should be listable for now but will get blocked by the stockroom guard
    return true if has_sku?

    # Regular logic:
    product_id.present? && sizing_id.present? && category_id.present? && brand_id.present? && gender.present?
  end

  def is_purchased?
    PURCHASED_ALL.include? self.state
  end

  def days_listed_before_purchase
    return 0 unless listed_on_site_since && purchased_at

    Time.difference(listed_on_site_since, purchased_at, 'days')
  end

  def seasons
    %w{winter spring summer fall}.select {|s| self.send(s) }
  end

  def colors
    return [] unless self[:colors]

    self[:colors].split(",")
  end

  def shortcuts
    look_ids = LookPail.where(pail_id: self.pail_ids).pluck(:look_id)
    Look.where(id: look_ids)
  end

  def to_json(options = {})
    super(options)
  end

  def as_json(options = {})
    # Also update the list in ItemCache, or these wont take effect
    options = {
      methods: [
        :availability,
        :category_name,
        :colors,
        :department,
        :description,
        :final_sale?,
        :first_listing_price,
        :in_cart,
        :item_brand,
        :item_colors,
        :item_sale,
        :item_size,
        :item_state,
        :long_description,
        :name,
        :new_without_tags,
        :orientation,
        :original_price,
        :path,
        :photo_urls,
        :price,
        :reserved,
        :savings,
        :sizing_and_scale
      ],
      include: [:brand]
    }.merge(options)

    super(options)
  end

  def to_cache
    item = ItemCache.new(self)
    item.fetch
  end

  def self.format_material_data(data = [])
    data.each_with_object([]) do |material, acc|
      next unless material.is_a?(Hash)

      mat_name = material['name']
      mat_percent = mat_name == 'No Fabric Content' ? '' : material['percent']

      acc << "#{mat_percent} #{mat_name}".strip
    end.join('|')
  end

  def sizing_type_name
    self.sizing.type_name
  end

  def generate_default_item_score
    return if !self.default_score

    if is_part_of_higher_scored_pails
      self.set_to_highest_score
    else
      self.default_score.calculate!
    end
  end

  def is_part_of_higher_scored_pails
    should_score_higher = false
    pail_ids = ItemPail.where(item_id: self.id).pluck(:pail_id)
    pail_ids.keep_if {|p_id| HIGHER_SCORED_PAIL_IDS.include? p_id}
    should_score_higher = true if pail_ids.count > 0
  end

  def calculate_score
    generate_default_item_score
  end

  def default_score
    default_item_score = self.item_scores.where(label: :default).first
    if !default_item_score
      self.default_score = ItemScore.create(item_id: self.id)
    else
      default_item_score
    end
  end

  def handle_price_admin_price_update!
    # for ops price admin app. Award credits to user if we increase price of an
    # item that we have already paid out.
    recalculate_price_and_final_sale
    save
    reload
    determine_sourcing_policy if consignment?
    save
  end

  def recalculate_price_and_final_sale(is_assign_final_sale = false)
    has_required_fields = brand_id.present? &&
                          category_id.present? &&
                          sizing_id ||
                          item_price.try(:is_msrp_overridden)
    return unless has_required_fields

    begin
      Item.transaction do
        # if calculator didn't worked, we should not proceed
        return false unless ItemCalculator.new(self).update!
        if is_assign_final_sale
          assign_final_sale
        else
          update_final_sale!
        end
      end
    rescue Exception => e
      Rollbar.warn(e, 'Exception occurred while recalculating price and final sale', attributes)
      raise e
    end
    publish_item_price_event
    self
  end

  def dry_recalculate_price(multipliers = {})
    has_required_fields = brand_id.present? &&
      category_id.present? &&
      sizing_id ||
      item_price.try(:is_msrp_overridden)
    return unless has_required_fields

    price_calculator = PriceCalculatorDry.new(self)
    price_calculator.multipliers[:exception] = multipliers[:exception] if multipliers[:exception]

    item_calculator = ItemCalculator.new(self)
    item_calculator.calculator = price_calculator
    item_calculator.to_item_price.price
  end

  def item_payout
    ItemPayout.new(self)
  end

  def calculate_upfront_payout!
    return if self.paid_out

    calculate_upfront_payout
    self.save
  end

  def category_id=(cid)
    cat = Category.find_by_id(cid)
    if cat
      self.winter = cat.winter
      self.spring = cat.spring
      self.summer = cat.summer
      self.fall   = cat.fall

      self[:category_id] = cid
    end
  end

  def measurement
    mappings = { "italian" => "IT",
                 "french" => "FR",
                 "eur" => "EUR",
                 "uk" => "UK",
                 "jap" => "JAP",
                 "aus" => "AUS",
                 "Chicos" => "Chicos",
                 "Zara" => "Zara",
                 "H&M" => "H&M",
                 "waist" => "Waist" }

    if sizing.try(:measurement_scale) && abbr = mappings[sizing.measurement_scale]
      "(#{abbr})"
    else
      ""
    end
  end

  # This is for mobile's rich HTML description on the SIV
  # iOS < 4.3 | iOS >=4.3 ==> ItemController#details_for_webview
  def long_description
    cond_desc = if new_with_tags
      '<li><b>Condition:</b> New With Tags</li>'
    elsif q2? || q3?
      '<li><b>Condition:</b> Tiny Flaw</li>'
    else
      '<li><b>Condition:</b> Excellent</li>'
    end

    char_desc = characteristics.map do |characteristic|
      "<li><b>#{characteristic.name.titleize}:</b> #{characteristic.value.titleize}</li>"
    end.join

    "<div class='details'><b><u>Details:</u></b><ul>#{cond_desc}<li><b>Quantity:</b> 1</li>#{char_desc}</ul></div>"
  end

  def price
    return @price if @price

    product_base_price = self.product.try(:base_price)
    if product_base_price
      @price = currencify(product_base_price.to_f / 100)
    end
  end

  def applicable_surcharges_for(user)
    surcharges = []

    if being_reclaimed_by_user?(user)
      surcharge = price_surcharges.reclaim_fee.first
      surcharges << surcharge if surcharge
    elsif logistics_fee_applicable?(user)
      surcharge = price_surcharges.logistics.first # New surcharge for all items regardless of user's warehouse assignments
      surcharges << surcharge if surcharge
    elsif !within_warehouse_for_user?(user) # Old MDC surcharge logic that will be deprecated soon
      surcharge = price_surcharges.mdc.first
      surcharges << surcharge if surcharge
    end

    surcharges
  end

  def logistics_fee_applicable?(user)
    return false if being_reclaimed_by_user?(user)

    # Reservation info stored for the item in Feature Store contains the base price of the item
    # and the price that user saw when adding the item to the cart.
    # Therefore we can tell the I.O. surcharge was applied by check that the difference
    # of price - item_price is 200
    surcharge_on_reservation(user) == 200
  rescue
    false
  end

  def user_adjusted_price?
    item_price.user_adjusted_price.present?
  end

  def within_warehouse_for_user?(user)
    return false if user.blank?

    user.warehouse_shopping_access.include?(warehouse_id)
  end

  def being_reclaimed_by_user?(user)
    is_buyer_seller_and_item_consignment?(user) && reclaim_in_progress?
  end

  def applicable_surcharges_for_preorder(preorder)
    [].tap do |surcharges|
      if preorder.surcharge_applied && price_surcharges.logistics.first
        surcharges << price_surcharges.logistics.first
      end
    end
  end

  def original_price
    return @original_price if @original_price

    if self.msrp
      @original_price = currencify(self.msrp.to_i/100)
    end
  end

  def get_category_group_uuids
    CategoryGroup.fetch_uuids_for_category(self.category_id) || []
  rescue Exception => e
    if Rails.env.production?
      detail = { item: self.id }
      Rollbar.error(e, detail)
    else
      ap e.inspect
      ap e.backtrace
    end
  end

  def get_root_node(forest, group)
    forest.select { |tree| tree.uuid == group.root_uuid }.compact.first
  end

  def first_sold_from_bag?
    first_sold = OrderProduct.paid.joins(:item).where(items: { concierge_bag_id: self.concierge_bag.try(:id) }).order("orders.purchased_at ASC, order_products.created_at ASC").first.try(:item_id)
    first_sold == self.id
  end

  def auto_generate_characteristics
    ItemCharacteristicAutoGenerator.new(self).run!
  end

  def assign_to_q3_pail
    self.add_to_pail(Pail::Q3_PAIL_ID) if self.womens? && self.q3? && Pail::Q3_PAIL_ID
  end

  def set_new_without_tags
    self.new_without_tags = is_new_without_tags?
  end

  def is_new_without_tags?
    is_one_jackson_wholesale? || (is_stitchfix_wholesale? && !new_with_tags)
  end

  def is_one_jackson_wholesale?
    (!self.sku.blank? && self.brand_id == 7919)
  end

  def is_stitchfix_wholesale?
    !self.concierge_bag || (self.concierge_bag.partner_id == 23)
  end

  ### This is primarily for positioning the icons correctly in shop - when both icons appear we need to add some custom positioning for the second.
  def nwt_and_flawed
    self.new_with_tags && self.q2?
  end

  def measurement_scale
    self.sizing.measurement_scale
  end

  def first_listing_price
    self.item_price.try(:original_price)
  end

  def formatted_first_listing_price
    return unless first_listing_price

    currencify(first_listing_price / 100.00, { currency_symbol: "$" })
  end

  # used for mobile clients
  def category_name
    return nil unless self.category

    begin
      name_parts = self.category.siv_display_name.split(" ").collect { |title_part| title_part.capitalize }

      return name_parts.join(" ")
    rescue
      return self.category.siv_display_name.titleize
    end
  end

  def inseam
    characteristics.inseam
  end

  def sizing_and_scale
    "#{size_label.to_s} #{measurement}".strip
  end

  def add_to_pail(pail_id)
    generate_item_pail(pail_id, true)
    self.set_to_highest_score if HIGHER_SCORED_PAIL_IDS.include? pail_id
    ItemMassPartialIndexingJob.update_item_index_if_changed([self], ["pails"])
  end

  def generate_item_pail(pail_id, update_pail_timestamp=false)
    item_pails = ItemPail.where(item_id: self.id).where(pail_id: pail_id)
    if item_pails.count == 0
      ItemPail.create(item_id: self.id, pail_id: pail_id)
      update_pail_updated_timestamp(pail_id) if update_pail_timestamp
    end
  end

  def remove_from_pail(pail_id)
    item_pails = ItemPail.where(item_id: self.id).where(pail_id: pail_id)
    item_pails.destroy_all if item_pails
    update_pail_updated_timestamp(pail_id)
    # Recalculate this item's score if it was removed from a pail that scores items higher
    self.calculate_score if HIGHER_SCORED_PAIL_IDS.include? pail_id
    ItemMassPartialIndexingJob.update_item_index_if_changed([self], ["pails"])
  end

  def update_pail_updated_timestamp(pail_id)
    pail = Pail.find_by_id(pail_id)
    pail.touch if pail # Update the timestamp for ordering pails in the curation menu
  end

  def set_to_highest_score
    default_item_score = self.default_score
    return if !default_item_score || (default_item_score && default_item_score.score == HIGHEST_ITEM_SCORE)

    default_item_score.update_attributes(score: HIGHEST_ITEM_SCORE)
  end

  def mw_json
    { item: {
          id: self.id,
          quality_code: self.quality_code,
          new_without_tags: self.new_without_tags,
          new_with_tags: self.new_with_tags,
          photo_urls: { medium_url: self.photo_urls.medium_url },
          path: self.path,
          state: self.state,
          price: self.price,
          savings: self.savings,
          brand: { name: self.brand.try(:name) },
          sizing_and_scale: self.sizing.measurement_1
        }
      }
  end

  def cached_url
    self.class.cached_url(item_number)
  end

  def self.cached_url(item_number)
    return '' unless item_number.present?

    Rails.cache.fetch("item_path_for_item_#{item_number}", expires_in: 200.days) { NewShopPdpUrlBuilder.new(item_number).build }
  end

  def bag_details_potential_consignment_earnings
    return self.consignment_payout_estimate if !self.paid_out

    return 0
  end

  def potential_consignment_earnings
    [ConsignmentPayoutCheck, PaidOutPayoutCheck, StatePayoutCheck].each do |validator|
      return 0 unless validator.validate(self)
    end
    return self.consignment_payout_estimate
  end

  def update_refunded_item!
    notify_erp_refunded
  end

  def notify_erp_refunded
    Async::Sidekiq::NotifyOps.perform_async(item_number, 'item_refunded', reason)
    self
  end

  def notify_erp_not_refunded
    Async::Sidekiq::NotifyOps.perform_async(item_number, 'item_not_refunded', reason)
    self
  end

  def belongs_to_admin_relist_bag?
    concierge_bag_id == ConciergeBag.admin_relist_bag_id
  end

  def misplaced?
    return false unless item_inventory_statuses.any?

    item_inventory_statuses.last.status == ItemInventoryStatus::LOST
  end

  def can_be_marked_as_reclaimable_by_cs?
    CAN_BE_MARKED_AS_RECLAIMABLE_BY_CS.include?(self.state) && item_flags.for_reclaim.active.blank?
  end

  def item_box_item?
    !!order_product.try(:order).try(:item_box?)
  end

  def order_in_terminal_state?
    item_box_item? && order_product.try(:order).try(:payment_declined_term?)
  end

  def should_appear_as_sold?
    lost? || to_be_transferred? || to_be_scrapped? || transferred?
  end

  def flagged?
    item_flags.for_review.active.exists?
  end

  def inappropriate_content?
    item_flags.inappropriate.exists?
  end

  def clear_stockroom_reason
    self.stockroom_reason = nil
  end

  def reservation(requesting_user:)
    return if !reserved? || requesting_user.blank?

    response = CartBizApi.new(requesting_user, s2s: true).get_item_cart_product(item_number)
    response.fetch(:cart_product, {})
  rescue CartBizApi::CartBizApiError
    nil
  end

  # This is a very expensive call. Do not use it unless it is absolutely
  # necessary
  def reserved_by_user?(user)
    return false if !reserved? || user.blank?

    cart = CartBizApi.new(user).get_cart
    cart.present? && (cart[:cart_products].any? { |cart_product| cart_product[:item_number] == item_number })
  rescue CartBizApi::CartBizApiError
    false
  end

  def assign_base_discount_rate_lookup_version!
    ItemBaseDiscountRateLookupVersion.where(
      item_number: item_number,
      base_discount_rate_lookup_version: allocate_list_price_experiment_version
    ).first_or_create
  end

  def allocate_list_price_experiment_version
    return 'v4' unless item_number

    (item_number % 100 < 20) ? 'v4' : 'v5'
  end

  # n = 0 gives primary material, n = 1 gives secondary material, and so on....
  def ordered_material(n)
    materials.to_s.split('|')[n].to_s.match(/% (.*)/).try(:[], 1)
  end

  def premium_or_designer?
    brand_tier_name.in?(BrandTier::PREMIUM_DESIGNER_TIER_NAMES)
  end

  def brand_tier_name
    Rails.cache.fetch("brand_tier_for_#{gender}_item_and_brand_#{brand_id}", expires_in: 14.days) do
      if gender.in?(['women', 'men'])
        (brand.adult_brand_tier || brand.brand_tier).name
      elsif gender.in?(['boys', 'girls'])
        (brand.brand_tier || brand.adult_brand_tier).name
      end
    end
  end

  def has_fall_booster?
    Rules::HasFallPayoutBooster.call(self)
  end

  def outlet_item?
    outlet_item.present?
  end

  def participate_in_experiment(experiment)
    item_experiments.where(experiment: experiment).first_or_create
  end

  def warehouse_code
    Rails.cache.fetch("warehouse:#{warehouse_id}:code", expires_in: CacheFlow.new.generate_expiry) do
      warehouse.code
    end
  end

  def can_payout?
    return false if is_raas_and_consignment_listing_window_has_completed?

    !in_superbag? && !ever_paid_out? && !reclaimed?
  end

  def ever_paid_out?
    return paid_out? unless derivative_item?
    original_item.paid_out? || original_item.derivative_items.paid_out.exists?
  end

  def paid_out_previously?
    return paid_out? unless derivative_item?
    original_item.paid_out? || original_item.derivative_items.paid_out.where("item_number < #{item_number}").exists?
  end

  def derivative_item?
    original_item_number.present? && item_number != original_item_number
  end

  def original_item?
    (original_item_number == item_number) || original_item_number.nil?
  end

  def original_item_record
    return self if original_item?

    original_item
  end

  def listing_date_adjustable?
    %w[reserved stockroom listed under_review reclaimable].include?(state)
  end

  def original_time_spent_by_state
    return time_spent_by_state_record if original_item_number == item_number || original_item_number.nil?
    original_item.time_spent_by_state_record
  end

  def record_payout
    item_payout.record_payout if consignment? && can_payout?
  end

  def assign_zero_payout_item
    return if zero_payout_item.present? || !ZeroPayoutRules::Matcher.call(self)

    rule = ZeroPayoutRules::Matcher.rule(self)
    create_zero_payout_item!(zero_payout_rule: rule)
  end

  def zero_payout_eligible?(force_lookup: false, net_amount: nil)
    return true if zero_payout_item.present?
    # do not allow the value to change after purchase when we persist the ZPI record
    (purchased_at.nil? || force_lookup) && ZeroPayoutRules::Matcher.call(self, net_amount: net_amount)
  end

  def partner_credit?
    partner&.award_method == Partner::PARTNER_CREDIT
  end

  def exclusive?
    !!item_exclusive_listing&.active
  end

  def should_receive_raas_mens_upfront_award?
    return false if paid_out? || in_superbag?
    return true if upfront? && mens? && has_accepted_mens_upfront_campaign_sourcing_policy?
  end

  def dropshipping?
    dropshipping_warehouse_id.present?
  end

  def supplier_id
    concierge_bag&.user_id
  end

  private

  def set_reclaim_item_product
    return unless self.consignment?

    op = OrderProduct.includes(:product).where(item_id: self.id).first
    self.update_attributes(product_id: op.product_id) if op.product.reclaimed?
  end

  def set_consignment_payout
    Item::ItemPayout.apply_consignment_payout!(self)
  end

  def update_price_on_purchase
    return if order_product.nil? || order_product.item_price == 0 || order_product.product&.reclaimed?

    if item_price.price != order_product.item_price
      Item.transaction do
        discount = 1 - (order_product.item_price / msrp.to_f).round(5)

        item_price.version = "purchase_price"
        item_price.override_discount_rate!(discount, true, false)
        @price = nil

        save
      end
    end
  rescue StandardError => e
    raise e if Rails.env.test?
    Rollbar.warn(e, 'Failed to update price on purchase')
  end

  def freeze_price
    item_price.try(:freeze!)
  end

  def log_state(event_source: nil)
    now = Time.now
    hist = self.history.create(state: state)
    hist.add_source(event_source) if event_source
    set_listing_details(now)
    set_purchased_at_stamp(now)
    set_stockroom_at_stamp(now)
    self.save
  end

  def ensure_color_names_are_present
    if !color_names.present? && !derivative_item? && listed_at && listed_at > 1.month.ago
      if Rails.env.production?
        details = { message: "An item came to web with no color_names Item ##{id}", item: id }
        Rollbar.error(e, details)
      end
    end
  end

  def set_listing_details(timestamp)
    if (self.listed? || self.listed_at_store?)
      if self[:listed_at].nil?
        self.listed_at = timestamp
        self.sorting_from = timestamp
      end
      self.item_price.update_attributes(listed_price: self.item_price.price) unless self.item_price.listed_price
    end
  end

  def set_stockroom_at_stamp(timestamp)
    if self.stockroom? && self[:stockroom_at].nil?
      self.stockroom_at = timestamp
    end
  end

  def backfill_stockroom_time
    if stockroom_at.nil?
      self.stockroom_at = Time.current
      save
    end
  end

  def set_purchased_at_stamp(timestamp)
    if (self.purchased? || self.packed?) && self[:purchased_at].nil?
      self.purchased_at = timestamp
    end
  end

  def ensure_price_consistency
    return unless VARIABLE_PRICE_STATES.include?(self.state)

    monitored_changes = self.changed & FIELDS_AFFECTING_PRICE
    if monitored_changes.count > 0 && self.persisted?
      is_complete = recalculate_price_and_final_sale(true)
      if is_complete && !paid_out && !being_relisted
        update_payout_if_bag_processed
      end
    end
  end

  def populate_size_label
    reset_size_label if size_label.blank? && sizing.present?
  end

  def reset_size_label
    return if self.size_label_overridden && !self.size_label.blank?

    self.size_label = self.sizing.get_web_name if self.sizing
    self.size_label << " #{self.sizing_modifier.name}" if self.sizing_modifier
    self.size_label
  end

  def restrict_to_final_sale
    if !override_final_sale.nil? && final_sale.blank?
      errors.add(:override_final_sale, "only final sale items can update this attribute.")
    end
  end

  def wholesale_data_valid_for_stockroom_to_be_transferred?
    return true unless upfront_wholesale?

    partner.award_method == Partner::INVOICE_BASED && price.present? && upfront_payout.present?
  end

  def update_payout_if_bag_processed
    self.concierge_bag.update_payout! if self.concierge_bag.processed?
  end

  def assign_item_number
    self.update_attributes(item_number: self.id)
  end

  def order_canceled?
    order.canceled?
  end

  class ConsignmentPayoutCheck
    def self.validate(item)
      item.consignment?
    end
  end
  class PaidOutPayoutCheck
    def self.validate(item)
      !item.paid_out
    end
  end
  class StatePayoutCheck
    def self.validate(item)
      item.listed? || item.reserved? || item.purchased?
    end
  end

  def create_partner_consignment_award
    return if !concierge_bag || (item_box_item? && !packed?)

    self.concierge_bag.create_partner_consignment_award(self)
  end

  def buyout_consignment_item_for_store
    return unless eligible_for_consignment_buyout?

    buyout_consignment_item(ItemTransfer::STORE_BUYOUT)
  end

  def ensure_stockroom_at
    self.stockroom_at = Time.now if self.stockroom_at.nil?
  end

  def set_ready_to_list_at
    return if ready_to_list_at.present?

    self.ready_to_list_at = Time.now
  end

  def create_delayed_listing_job
    Async::Sidekiq::ListDuplicateItem.perform_in(15.minutes, id) if stockroom?
  end

  def set_reitemization_listing_hold
    self.listing_hold = REITEMIZATION_LISTING_HOLD unless listing_hold?
  end

  def self.datadog_client
    @datadog_client ||= DatadogStatsdClient.new
  end

  def nullify_listed_price
    return unless listed_price_nullable?

    item_price.update(listed_price: nil)
  end

  def listed_price_nullable?
    return false if item_price.blank?

    (aasm.from_state == :under_review && aasm.to_state == :stockroom) || aasm.to_state == :destroyed_processing
  end

  def can_list_reclaimable?
    self.concierge_bag&.user_id != User::SUPER_USER_ID
  end
end
