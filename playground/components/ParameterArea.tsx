import * as React from "react";
import { useState, useEffect } from "react";
import axios from "axios";
import { AxiosRequestConfig } from "axios";
import { useRouter } from "next/router";

const serverConfig = {
  url: process.env.NEXT_PUBLIC_SERVER_URL || "",
};

type Props = {
  fetchData: (
    fetchUrl: string,
    option: { offset: string; limit: string; row: string },
  ) => void;
  searching: boolean;
};

const ParameterArea: React.FC<Props> = ({ fetchData, searching }) => {
  const router = useRouter();
  let [sheetUrlVal, setSheetUrlVal] = useState("");
  let [offsetVal, setOffsetVal] = useState("");
  let [limitVal, setLimitVal] = useState("");
  let [rowVal, setRowVal] = useState("");
  let [serviceAccount, setServiceAccount] = useState("");

  const inputUrl = (e: any) => {
    setSheetUrlVal(e.target.value);
  };

  const inputOffset = (e: any) => {
    setOffsetVal(e.target.value);
  };

  const inputLimit = (e: any) => {
    setLimitVal(e.target.value);
  };

  const inputRow = (e: any) => {
    setRowVal(e.target.value);
  };

  useEffect(() => {
    const { sheetUrl } = router.query;
    if (sheetUrl) {
      setSheetUrlVal(decodeURIComponent(sheetUrl as string));
    }
  }, [router]);

  useEffect(() => {
    //TODO(tacogips) TOBE DRY
    async function loadServiceAccount() {
      const config = {
        url: `${serverConfig.url}/meta`,
        method: "get",
      } as AxiosRequestConfig;

      const result = await axios.request(config).catch(error => {
        if (error.response) {
          if (error.response.status == 404) {
            return null;
          } else if (error.response.status != 200) {
            return null;
          }
        }
        return null;
      });
      if (result) {
        setServiceAccount(result.data.service_account);
      }
    }
    loadServiceAccount();
  }, []);

  return (
    <>
      <div className="w-full ">
        <form className="bg-white shadow-md rounded px-8 pt-6 pb-8 mb-4">
          <div className="mb-4">
            <label
              className="block text-gray-700 text-sm font-bold mb-2"
              htmlFor="spread_sheet_url"
            >
              spread sheet url
              <br /> (published or share with `{serviceAccount}`)
            </label>
            <input
              className="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
              id="spread_sheet_url"
              type="text"
              placeholder="ex. https://docs.google.com/spreadsheets/d/your_spread_sheet_id/edit#gid=0"
              value={sheetUrlVal}
              onChange={inputUrl}
            />
          </div>
          <div className="flex">
            <label
              className="block text-gray-700 text-sm font-bold py-3"
              htmlFor="offset"
            >
              offset(option)
            </label>
            <input
              className="w-24 shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
              id="offset"
              type="text"
              placeholder="0"
              value={offsetVal}
              onChange={inputOffset}
            />

            <label
              className="block text-gray-700 text-sm font-bold pl-3 py-3"
              htmlFor="limit"
            >
              limit(option)
            </label>
            <input
              className="w-24 shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
              id="limit"
              type="text"
              placeholder="100"
              value={limitVal}
              onChange={inputLimit}
            />

            <label
              className="block text-gray-700 text-sm font-bold pl-3 py-3"
              htmlFor="row"
            >
              row(option)
            </label>
            <input
              className="w-24 shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
              id="row"
              type="text"
              placeholder="100"
              value={rowVal}
              onChange={inputRow}
            />
          </div>
          <div
            className="bg-red-100 border border-red-400 text-red-700 my-5 px-4 py-2 rounded relative"
            role="alert"
          >
            <strong className="font-bold">
              Spread sheet MUST NOT Contains sensitive datas.
            </strong>
            <span className="block inline">
              This tool and API are published with{" "}
              <span className="underline">
                <a href="http://github.com/tacogips/api-everywhere/LISENCE">
                  MIT Licence
                </a>
              </span>
              , so use it at your own risk.
            </span>
          </div>
          {searching ? (
            <span className="bg-blue-200  text-white font-bold py-2 px-3 rounded">
              Seaching. Please wait.
            </span>
          ) : (
            <span
              className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-3 rounded"
              onClick={() =>
                fetchData(sheetUrlVal, {
                  offset: offsetVal,
                  limit: limitVal,
                  row: rowVal,
                })
              }
            >
              Fetch Data
            </span>
          )}
        </form>
      </div>
    </>
  );
};

export default ParameterArea;
