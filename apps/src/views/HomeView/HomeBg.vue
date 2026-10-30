<template>
  <div class="energy-bg">
    <svg
      class="svg"
      width="1310"
      height="569"
      viewBox="0 0 1310 569"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      xmlns:xlink="http://www.w3.org/1999/xlink"
    >
      <mask
        id="mask0_1172_2531"
        style="mask-type: alpha"
        maskUnits="userSpaceOnUse"
        x="0"
        y="0"
        width="1310"
        height="569"
      >
        <rect width="1310" height="569" fill="#D9D9D9" />
      </mask>
      <g mask="url(#mask0_1172_2531)">
        <foreignObject x="-220" y="-196" width="1920" height="1080" viewBox="0 0 1920 1080">
          <div xmlns="http://www.w3.org/1999/xhtml" class="bg"></div>
        </foreignObject>
        <g :filter="flow.pv ? 'url(#filter0_ddi_1172_2531)' : undefined">
          <path
            d="M565.096 190.185L578.662 184.137L693.482 249.138L693.482 263.638L565.096 190.185Z"
            :fill="flow.pv ? 'url(#paint0_linear_1172_2531)' : '#54628C'"
          />
        </g>

        <g
          transform="rotate(28.4666 620.906 204.24)"
          clip-path="url(#arrow-clip-1)"
          class="svg-arrow-track"
          data-energy-flow="pv"
          :data-direction="flow.pv"
        >
          <g
            v-if="flow.pv"
            :key="flow.pv"
            :transform="flow.pv < 0 ? 'translate(1302.292 0) scale(-1 1)' : undefined"
          >
            <g class="svg-arrow-motion">
              <use href="#arrow-shape" x="620.906" y="204.24" />
              <use href="#arrow-shape" x="681.386" y="204.24" />
              <animateTransform
                attributeName="transform"
                type="translate"
                from="-60.48 0"
                to="0 0"
                dur="2.4s"
                repeatCount="indefinite"
              />
            </g>
          </g>
        </g>
        <g filter="url(#filter1_f_1172_2531)">
          <circle
            cx="583.457"
            cy="189.097"
            r="21.6"
            transform="rotate(-2.32068 583.457 189.097)"
            fill="black"
          />
        </g>
        <g :filter="flow.diesel ? 'url(#filter2_ddi_1172_2531)' : undefined">
          <path
            d="M856 371.235L959.5 430.5V416L869.2 365L856 371.235Z"
            :fill="flow.diesel ? 'url(#paint1_linear_1172_2531)' : '#54628C'"
          />
        </g>
        <g :filter="flow.load ? 'url(#filter3_ddi_1172_2531)' : undefined">
          <path
            d="M744.018 348L658.2 402.243L658.2 403L658.2 417L756.5 354.5L744.018 348Z"
            :fill="flow.load ? 'url(#paint2_linear_1172_2531)' : '#54628C'"
          />
        </g>
        <g
          transform="rotate(148.574 735.265 373.892)"
          clip-path="url(#arrow-clip-2)"
          class="svg-arrow-track"
          data-energy-flow="load"
          :data-direction="flow.load"
        >
          <g
            v-if="flow.load"
            :key="flow.load"
            :transform="flow.load < 0 ? 'translate(1531.01 0) scale(-1 1)' : undefined"
          >
            <g class="svg-arrow-motion">
              <use href="#arrow-shape" x="735.265" y="373.892" />
              <use href="#arrow-shape" x="795.745" y="373.892" />
              <animateTransform
                attributeName="transform"
                type="translate"
                from="-60.48 0"
                to="0 0"
                dur="2.4s"
                repeatCount="indefinite"
              />
            </g>
          </g>
        </g>
        <g
          transform="rotate(31.19 890.314 370.921)"
          clip-path="url(#arrow-clip-3)"
          class="svg-arrow-track"
          data-energy-flow="diesel"
          :data-direction="flow.diesel"
        >
          <g
            v-if="flow.diesel"
            :key="flow.diesel"
            :transform="flow.diesel < 0 ? 'translate(1841.108 0) scale(-1 1)' : undefined"
          >
            <g class="svg-arrow-motion">
              <use href="#arrow-shape" x="890.314" y="370.921" />
              <use href="#arrow-shape" x="950.794" y="370.921" />
              <animateTransform
                attributeName="transform"
                type="translate"
                from="-60.48 0"
                to="0 0"
                dur="2.4s"
                repeatCount="indefinite"
              />
            </g>
          </g>
        </g>
        <g filter="url(#filter4_f_1172_2531)">
          <circle
            cx="866.457"
            cy="372.457"
            r="21.6"
            transform="rotate(-2.32068 866.457 372.457)"
            fill="black"
          />
        </g>
        <g filter="url(#filter5_f_1172_2531)">
          <circle
            cx="740.457"
            cy="356.457"
            r="21.6"
            transform="rotate(-2.32068 740.457 356.457)"
            fill="black"
          />
        </g>
        <path
          data-figma-bg-blur-radius="10"
          d="M83 38.5H202.5V118C202.5 128.77 193.77 137.5 183 137.5H63.5V58C63.5 47.2304 72.2304 38.5 83 38.5Z"
          fill="#54628C"
          fill-opacity="0.5"
          stroke="url(#paint4_linear_1172_2531)"
        />
        <g class="svg-card-text">
          <text x="83" y="76" class="svg-card-title">PV</text>
          <text x="83" y="106" class="svg-card-value">
            <tspan>{{ props.data.pv.P?.name }}:</tspan>
            <tspan dx="8" class="svg-card-number">{{ props.data.pv.P?.value }}</tspan>
            <tspan dx="5" class="svg-card-unit">{{ props.data.pv.P?.unit }}</tspan>
          </text>
        </g>

        <circle
          cx="306"
          cy="121"
          r="6"
          fill="white"
          fill-opacity="0.8"
          stroke="white"
          stroke-opacity="0.2"
          stroke-width="20"
        />
        <path
          d="M203 95H285L305.5 119"
          stroke="url(#paint5_linear_1172_2531)"
          stroke-opacity="0.3"
        />
        <path
          data-figma-bg-blur-radius="10"
          d="M121 294.5H240.5V374C240.5 384.77 231.77 393.5 221 393.5H101.5V314C101.5 303.23 110.23 294.5 121 294.5Z"
          fill="#54628C"
          fill-opacity="0.5"
          stroke="url(#paint6_linear_1172_2531)"
        />
        <g class="svg-card-text">
          <text x="121" y="332" class="svg-card-title">Load</text>
          <text x="121" y="362" class="svg-card-value">
            <tspan>{{ props.data.load.P?.name }}:</tspan>
            <tspan dx="8" class="svg-card-number">{{ props.data.load.P?.value }}</tspan>
            <tspan dx="5" class="svg-card-unit">{{ props.data.load.P?.unit }}</tspan>
          </text>
        </g>
        <!-- <circle cx="234" cy="309" r="15" fill="#DA2D2C" />
        <path
          d="M233.999 304.051C237.23 304.051 239.849 306.67 239.849 309.901V314.401H231.749V310.351H230.399V314.401H228.149V309.901C228.149 306.67 230.768 304.051 233.999 304.051ZM227.831 303.003L229.422 304.594L228.468 305.548L226.876 303.958L227.83 303.003H227.831ZM240.167 303.003L241.122 303.957L239.531 305.548L238.576 304.594L240.167 303.003ZM234.674 300.901V303.151H233.324V300.901H234.674ZM227.249 315.571H240.749V316.921H227.249V315.571Z"
          fill="white"
        /> -->
        <circle
          cx="392"
          cy="378"
          r="6"
          fill="white"
          fill-opacity="0.8"
          stroke="white"
          stroke-opacity="0.2"
          stroke-width="20"
        />
        <path
          d="M241 352H371L391.5 376"
          stroke="url(#paint7_linear_1172_2531)"
          stroke-opacity="0.3"
        />
        <path
          data-figma-bg-blur-radius="10"
          d="M1133 396.5H1252.5V476C1252.5 486.77 1243.77 495.5 1233 495.5H1113.5V416C1113.5 405.23 1122.23 396.5 1133 396.5Z"
          fill="#54628C"
          fill-opacity="0.5"
          stroke="url(#paint8_linear_1172_2531)"
        />
        <g class="svg-card-text">
          <text x="1143" y="434" class="svg-card-title">Diesel</text>
          <text x="1143" y="461" class="svg-card-value">
            <tspan>{{ props.data.diesel.p?.name }}:</tspan>
            <tspan dx="8" class="svg-card-number">{{ props.data.diesel.p?.value }}</tspan>
            <tspan dx="5" class="svg-card-unit">{{ props.data.diesel.p?.unit }}</tspan>
          </text>
          <text x="1143" y="482" class="svg-card-value">
            <tspan>{{ props.data.diesel.oil?.name }}:</tspan>
            <tspan dx="8" class="svg-card-number">{{ props.data.diesel.oil?.value }}</tspan>
            <tspan dx="5" class="svg-card-unit">{{ props.data.diesel.oil?.unit }}</tspan>
          </text>
        </g>
        <circle
          cx="994"
          cy="428"
          r="6"
          transform="rotate(180 994 428)"
          fill="white"
          fill-opacity="0.8"
          stroke="white"
          stroke-opacity="0.2"
          stroke-width="20"
        />
        <path
          d="M1113 454H1015L994.5 430"
          stroke="url(#paint9_linear_1172_2531)"
          stroke-opacity="0.3"
        />
        <path
          data-figma-bg-blur-radius="10"
          d="M1036 180.5H1155.5V260C1155.5 270.77 1146.77 279.5 1136 279.5H1016.5V200C1016.5 189.23 1025.23 180.5 1036 180.5Z"
          fill="#54628C"
          fill-opacity="0.5"
          stroke="url(#paint10_linear_1172_2531)"
        />
        <g class="svg-card-text">
          <text x="1046" y="218" class="svg-card-title">ESS</text>
          <text x="1046" y="245" class="svg-card-value">
            <tspan>{{ props.data.ess.p?.name }}:</tspan>
            <tspan dx="8" class="svg-card-number">{{ props.data.ess.p?.value }}</tspan>
            <tspan dx="5" class="svg-card-unit">{{ props.data.ess.p?.unit }}</tspan>
          </text>
          <text x="1046" y="266" class="svg-card-value">
            <tspan>{{ props.data.ess.soc?.name }}:</tspan>
            <tspan dx="8" class="svg-card-number">{{ props.data.ess.soc?.value }}</tspan>
            <tspan dx="5" class="svg-card-unit">{{ props.data.ess.soc?.unit }}</tspan>
          </text>
        </g>
        <circle
          cx="6"
          cy="6"
          r="6"
          transform="matrix(-1 0 0 1 826 226)"
          fill="white"
          fill-opacity="0.8"
          stroke="white"
          stroke-opacity="0.2"
          stroke-width="20"
        />
        <path
          d="M1016 206H842L821.5 230"
          stroke="url(#paint11_linear_1172_2531)"
          stroke-opacity="0.3"
        />
      </g>
      <defs>
        <clipPath id="arrow-clip-1" clipPathUnits="userSpaceOnUse">
          <path
            d="M627.906 204.24H674.386L681.386 214.8L674.386 225.36H627.906L620.906 214.8L627.906 204.24Z"
          />
        </clipPath>
        <clipPath id="arrow-clip-2" clipPathUnits="userSpaceOnUse">
          <path
            d="M742.265 373.892H788.745L795.745 384.452L788.745 395.012H742.265L735.265 384.452L742.265 373.892Z"
          />
        </clipPath>
        <clipPath id="arrow-clip-3" clipPathUnits="userSpaceOnUse">
          <path
            d="M897.314 370.921H943.794L950.794 381.481L943.794 392.041H897.314L890.314 381.481L897.314 370.921Z"
          />
        </clipPath>
        <path
          id="arrow-shape"
          d="M0 0H5L13 10.56L5 21H0L8 10.56ZM12.096 0H17.096L25.096 10.56L17.096 21H12.096L20.096 10.56ZM24.192 0H29.192L37.192 10.56L29.192 21H24.192L32.192 10.56ZM36.288 0H41.288L49.288 10.56L41.288 21H36.288L44.288 10.56ZM48.384 0H53.384L61.384 10.56L53.384 21H48.384L56.384 10.56Z"
          fill="#F8DB6B"
        />
        <filter
          id="filter0_ddi_1172_2531"
          x="529.096"
          y="160.137"
          width="200.385"
          height="151.501"
          filterUnits="userSpaceOnUse"
          color-interpolation-filters="sRGB"
        >
          <feFlood flood-opacity="0" result="BackgroundImageFix" />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset dy="12" />
          <feGaussianBlur stdDeviation="18" />
          <feComposite in2="hardAlpha" operator="out" />
          <feColorMatrix type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.25 0" />
          <feBlend mode="normal" in2="BackgroundImageFix" result="effect1_dropShadow_1172_2531" />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset />
          <feGaussianBlur stdDeviation="12" />
          <feComposite in2="hardAlpha" operator="out" />
          <feColorMatrix
            type="matrix"
            values="0 0 0 0 1 0 0 0 0 0.992066 0 0 0 0 0.72323 0 0 0 0.5 0"
          />
          <feBlend
            mode="normal"
            in2="effect1_dropShadow_1172_2531"
            result="effect2_dropShadow_1172_2531"
          />
          <feBlend
            mode="normal"
            in="SourceGraphic"
            in2="effect2_dropShadow_1172_2531"
            result="shape"
          />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset />
          <feGaussianBlur stdDeviation="6" />
          <feComposite in2="hardAlpha" operator="arithmetic" k2="-1" k3="1" />
          <feColorMatrix type="matrix" values="0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 1 0" />
          <feBlend mode="normal" in2="shape" result="effect3_innerShadow_1172_2531" />
        </filter>
        <filter
          id="filter1_f_1172_2531"
          x="501.856"
          y="107.497"
          width="163.201"
          height="163.201"
          filterUnits="userSpaceOnUse"
          color-interpolation-filters="sRGB"
        >
          <feFlood flood-opacity="0" result="BackgroundImageFix" />
          <feBlend mode="normal" in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="30" result="effect1_foregroundBlur_1172_2531" />
        </filter>
        <filter
          id="filter2_ddi_1172_2531"
          x="820"
          y="341"
          width="175.5"
          height="137.5"
          filterUnits="userSpaceOnUse"
          color-interpolation-filters="sRGB"
        >
          <feFlood flood-opacity="0" result="BackgroundImageFix" />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset dy="12" />
          <feGaussianBlur stdDeviation="18" />
          <feComposite in2="hardAlpha" operator="out" />
          <feColorMatrix type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.25 0" />
          <feBlend mode="normal" in2="BackgroundImageFix" result="effect1_dropShadow_1172_2531" />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset />
          <feGaussianBlur stdDeviation="12" />
          <feComposite in2="hardAlpha" operator="out" />
          <feColorMatrix
            type="matrix"
            values="0 0 0 0 1 0 0 0 0 0.992066 0 0 0 0 0.72323 0 0 0 0.5 0"
          />
          <feBlend
            mode="normal"
            in2="effect1_dropShadow_1172_2531"
            result="effect2_dropShadow_1172_2531"
          />
          <feBlend
            mode="normal"
            in="SourceGraphic"
            in2="effect2_dropShadow_1172_2531"
            result="shape"
          />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset />
          <feGaussianBlur stdDeviation="6" />
          <feComposite in2="hardAlpha" operator="arithmetic" k2="-1" k3="1" />
          <feColorMatrix type="matrix" values="0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 1 0" />
          <feBlend mode="normal" in2="shape" result="effect3_innerShadow_1172_2531" />
        </filter>
        <filter
          id="filter3_ddi_1172_2531"
          x="622.2"
          y="324"
          width="170.3"
          height="141"
          filterUnits="userSpaceOnUse"
          color-interpolation-filters="sRGB"
        >
          <feFlood flood-opacity="0" result="BackgroundImageFix" />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset dy="12" />
          <feGaussianBlur stdDeviation="18" />
          <feComposite in2="hardAlpha" operator="out" />
          <feColorMatrix type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.25 0" />
          <feBlend mode="normal" in2="BackgroundImageFix" result="effect1_dropShadow_1172_2531" />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset />
          <feGaussianBlur stdDeviation="12" />
          <feComposite in2="hardAlpha" operator="out" />
          <feColorMatrix
            type="matrix"
            values="0 0 0 0 1 0 0 0 0 0.992066 0 0 0 0 0.72323 0 0 0 0.5 0"
          />
          <feBlend
            mode="normal"
            in2="effect1_dropShadow_1172_2531"
            result="effect2_dropShadow_1172_2531"
          />
          <feBlend
            mode="normal"
            in="SourceGraphic"
            in2="effect2_dropShadow_1172_2531"
            result="shape"
          />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset />
          <feGaussianBlur stdDeviation="6" />
          <feComposite in2="hardAlpha" operator="arithmetic" k2="-1" k3="1" />
          <feColorMatrix type="matrix" values="0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 1 0" />
          <feBlend mode="normal" in2="shape" result="effect3_innerShadow_1172_2531" />
        </filter>
        <filter
          id="filter4_f_1172_2531"
          x="784.856"
          y="290.857"
          width="163.201"
          height="163.201"
          filterUnits="userSpaceOnUse"
          color-interpolation-filters="sRGB"
        >
          <feFlood flood-opacity="0" result="BackgroundImageFix" />
          <feBlend mode="normal" in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="30" result="effect1_foregroundBlur_1172_2531" />
        </filter>
        <filter
          id="filter5_f_1172_2531"
          x="658.856"
          y="274.857"
          width="163.201"
          height="163.201"
          filterUnits="userSpaceOnUse"
          color-interpolation-filters="sRGB"
        >
          <feFlood flood-opacity="0" result="BackgroundImageFix" />
          <feBlend mode="normal" in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="30" result="effect1_foregroundBlur_1172_2531" />
        </filter>
        <clipPath id="bgblur_0_1172_2531_clip_path" transform="translate(-658.999 -398)">
          <rect x="669.499" y="408.5" width="91" height="29" rx="14.5" />
        </clipPath>
        <clipPath id="bgblur_1_1172_2531_clip_path" transform="translate(-53 -28)">
          <path
            d="M83 38.5H202.5V118C202.5 128.77 193.77 137.5 183 137.5H63.5V58C63.5 47.2304 72.2304 38.5 83 38.5Z"
          />
        </clipPath>
        <clipPath id="bgblur_2_1172_2531_clip_path" transform="translate(-91 -284)">
          <path
            d="M121 294.5H240.5V374C240.5 384.77 231.77 393.5 221 393.5H101.5V314C101.5 303.23 110.23 294.5 121 294.5Z"
          />
        </clipPath>
        <clipPath id="bgblur_3_1172_2531_clip_path" transform="translate(-1103 -386)">
          <path
            d="M1133 396.5H1252.5V476C1252.5 486.77 1243.77 495.5 1233 495.5H1113.5V416C1113.5 405.23 1122.23 396.5 1133 396.5Z"
          />
        </clipPath>
        <clipPath id="bgblur_4_1172_2531_clip_path" transform="translate(-1006 -170)">
          <path
            d="M1036 180.5H1155.5V260C1155.5 270.77 1146.77 279.5 1136 279.5H1016.5V200C1016.5 189.23 1025.23 180.5 1036 180.5Z"
          />
        </clipPath>
        <linearGradient
          id="paint0_linear_1172_2531"
          x1="572.075"
          y1="184.588"
          x2="710.965"
          y2="263.029"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="#FF6900" />
          <stop offset="1" stop-color="#FE9900" />
        </linearGradient>
        <linearGradient
          id="paint1_linear_1172_2531"
          x1="856"
          y1="371.946"
          x2="984.4"
          y2="443.346"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="#FE9900" />
          <stop offset="1" stop-color="#FF6900" />
        </linearGradient>
        <linearGradient
          id="paint2_linear_1172_2531"
          x1="651"
          y1="417.6"
          x2="755.4"
          y2="348"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="#FF6900" />
          <stop offset="1" stop-color="#FE9900" />
        </linearGradient>
        <linearGradient
          id="paint3_linear_1172_2531"
          x1="671.628"
          y1="408.75"
          x2="709.176"
          y2="469.045"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="#94A6C5" />
          <stop offset="0.30251" stop-color="#94A6C5" stop-opacity="0" />
          <stop offset="0.718599" stop-color="#94A6C5" stop-opacity="0.344221" />
          <stop offset="0.990385" stop-color="#94A6C5" />
        </linearGradient>
        <linearGradient
          id="paint4_linear_1172_2531"
          x1="67"
          y1="40.5"
          x2="200"
          y2="138"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="#94A6C5" />
          <stop offset="0.30251" stop-color="#94A6C5" stop-opacity="0" />
          <stop offset="0.718599" stop-color="#94A6C5" stop-opacity="0.344221" />
          <stop offset="0.990385" stop-color="#94A6C5" />
        </linearGradient>
        <linearGradient
          id="paint5_linear_1172_2531"
          x1="267"
          y1="90.5"
          x2="294"
          y2="122.5"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="white" />
          <stop offset="1" stop-color="white" stop-opacity="0" />
        </linearGradient>
        <linearGradient
          id="paint6_linear_1172_2531"
          x1="105"
          y1="296.5"
          x2="238"
          y2="394"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="#94A6C5" />
          <stop offset="0.30251" stop-color="#94A6C5" stop-opacity="0" />
          <stop offset="0.718599" stop-color="#94A6C5" stop-opacity="0.344221" />
          <stop offset="0.990385" stop-color="#94A6C5" />
        </linearGradient>
        <linearGradient
          id="paint7_linear_1172_2531"
          x1="409"
          y1="350.5"
          x2="431.5"
          y2="379"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="white" />
          <stop offset="1" stop-color="white" stop-opacity="0" />
        </linearGradient>
        <linearGradient
          id="paint8_linear_1172_2531"
          x1="1117"
          y1="398.5"
          x2="1250"
          y2="496"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="#94A6C5" />
          <stop offset="0.30251" stop-color="#94A6C5" stop-opacity="0" />
          <stop offset="0.718599" stop-color="#94A6C5" stop-opacity="0.344221" />
          <stop offset="0.990385" stop-color="#94A6C5" />
        </linearGradient>
        <linearGradient
          id="paint9_linear_1172_2531"
          x1="1027.5"
          y1="456"
          x2="995"
          y2="427.5"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="white" />
          <stop offset="1" stop-color="white" stop-opacity="0" />
        </linearGradient>
        <linearGradient
          id="paint10_linear_1172_2531"
          x1="1020"
          y1="182.5"
          x2="1153"
          y2="280"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="#94A6C5" />
          <stop offset="0.30251" stop-color="#94A6C5" stop-opacity="0" />
          <stop offset="0.718599" stop-color="#94A6C5" stop-opacity="0.344221" />
          <stop offset="0.990385" stop-color="#94A6C5" />
        </linearGradient>
        <linearGradient
          id="paint11_linear_1172_2531"
          x1="855.5"
          y1="196"
          x2="822.5"
          y2="234.5"
          gradientUnits="userSpaceOnUse"
        >
          <stop stop-color="white" />
          <stop offset="1" stop-color="white" stop-opacity="0" />
        </linearGradient>
      </defs>
    </svg>
  </div>
</template>
<script setup lang="ts">
import { computed } from 'vue'

interface TopologyPoint {
  id?: number
  name?: string
  value?: unknown
  rawValue?: unknown
  unit?: string
}
const props = defineProps<{
  data: {
    pv: { P?: TopologyPoint }
    load: { P?: TopologyPoint }
    diesel: { p?: TopologyPoint; oil?: TopologyPoint }
    ess: { p?: TopologyPoint; soc?: TopologyPoint }
  }
}>()
// Use raw readings: formatted placeholders and rounded display values are not power.
function powerDirection(point?: TopologyPoint): number {
  const value = point?.rawValue
  if (typeof value !== 'number' && (typeof value !== 'string' || !value.trim())) return 0
  const power = Number(value)
  return Number.isFinite(power) ? Math.sign(power) : 0
}

const flow = computed(() => ({
  pv: powerDirection(props.data.pv.P),
  load: powerDirection(props.data.load.P),
  // This track is drawn from the central system towards the generator.
  // Positive generation must travel in the opposite direction.
  diesel: -powerDirection(props.data.diesel.p),
}))
</script>
<style lang="scss" scoped>
.energy-bg {
  width: 100%;
  height: 100%;

  // z-index: -100;
  .svg {
    width: 100%;
    height: 100%;
    display: block;
  }
}

.no-rem__card {
  position: relative;
  width: 100%;
  height: 100%;
  padding: 0 30px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 10px;
}

.no-rem__card-title {
  font-weight: 700;
  font-size: 18px;
  line-height: 18px;
  color: var(--vt-text-primary);
}

.no-rem__card-content {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 6px;
}

.no-rem__card-content-item {
  height: 16px;
  display: flex;
  align-items: flex-end;
}

.no-rem__name {
  font-weight: 400;
  font-size: 16px;
  color: var(--vt-text-primary);
  margin-right: 9px;
}

.no-rem__value {
  font-weight: 700;
  font-size: 16px;
  text-transform: capitalize;
  color: var(--vt-text-primary);
  vertical-align: bottom;
  margin-right: 4px;
}

.no-rem__unit {
  font-weight: 400;
  font-size: 12px;
  text-transform: capitalize;
  color: rgba(255, 255, 255, 0.5);
  vertical-align: bottom;
}

.no-rem__card-alarm {
  position: absolute;
  top: 10px;
  right: 0px;
  width: 30px;
  height: 30px;
  background-image: url('../../assets/icons/card-alarm.svg');
  background-size: 100% 100%;
  background-repeat: no-repeat;
  z-index: 999;
}

.horizontal-loop {
  width: 100%;
  height: 100%;
  position: relative;
  overflow: hidden;
  padding: 0;
  background: transparent;

  clip-path: polygon(0% 0%, calc(100% - 7px) 0%, 100% 50%, calc(100% - 7px) 100%, 0% 100%, 7px 50%);
  z-index: 999;
}

// .horizontal-loop.loop1 {
//     transform-origin: 489.714px 524.347px;
// }

// .horizontal-loop.loop2 {
//     transform-origin: 809.812px 571.209px;
// }

// .horizontal-loop.loop3 {
//     transform-origin: 1109.4px 606.91px;
// }

.horizontal-loop::before {
  content: '';
  position: absolute;
  left: -7px;
  top: 0;
  width: 7px;
  height: 100%;
  background: transparent;
  // clip-path: polygon(0% 0%, 100% 30%, 0% 100%);
  z-index: 1;
}

.horizontal-loop::after {
  content: '';
  position: absolute;
  right: -7px;
  top: 0;
  width: 7px;
  height: 100%;
  background: #1a1a1a;
  // clip-path: polygon(0% 50%, 100% 0%, 100% 100%);
  z-index: 10;
}

.arrow-container {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 100%;
  height: 100%;
}

.arrow-container img {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  object-fit: contain;
}

/* 创建多个箭头实现无缝效果 - 紧密排列 */
.arrow1 {
  animation: moveRight 2.4s linear infinite;
}

.arrow2 {
  animation: moveRight 2.4s linear infinite -1.2s;
}

@keyframes moveRight {
  0% {
    left: -60.48px;
  }

  100% {
    left: 60.48px;
  }
}

.bg {
  width: 100%;
  height: 100%;
  background-image: url('@/assets/images/tuopu-bg.png');
  background-size: cover;
  background-position: center;
  background-repeat: no-repeat;
  display: block;
}

.svg-card-title,
.svg-card-value,
.svg-card-number,
.svg-card-unit {
  font-family: Arimo, Arial, sans-serif;
  fill: var(--vt-text-primary);
}

.svg-card-title {
  font-size: 18px;
  font-weight: 700;
}

.svg-card-value {
  font-size: 16px;
  font-weight: 400;
}

.svg-card-number {
  font-weight: 700;
}

.svg-card-unit {
  font-size: 12px;
  fill: rgba(255, 255, 255, 0.5);
}

/* px-to-rem-enable */
</style>
